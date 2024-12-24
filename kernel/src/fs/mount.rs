use core::{alloc::Layout, ffi::{c_char, c_void, CStr}, ptr::{addr_of, addr_of_mut, null_mut}};
use proc_macro::__init; 
use alloc::{collections::{BTreeMap, BTreeSet, LinkedList}, string::String, sync::Arc};
use bitflags::bitflags;
use crate::{bit, container_of, kernel::{errno_base::{err_ptr, is_err, ptr_err, EBUSY, EFAULT, EINVAL, EISDIR, ENOMEM, ENOSPC, ENOTDIR, EPERM}, list::ListHead, Err}, fs::{pnode::set_mnt_shared, fs::{SB_RDONLY, SB_SYNCHRONOUS, SB_MANDLOCK, SB_DIRSYNC, SB_SILENT, SB_POSIXACL, SB_LAZYTIME, SB_I_VERSION, FileSystemType}}};
use crate::mm::memory::PAGE_SIZE;
use super::{dcache::{DEntry, DEntryFlags}, file::{LogicalPart, FS}, fs::{SB_I_NODEV, SB_I_NOEXEC, SB_I_USERNS_VISIBLE, SB_NOUSER}, fs_context::{vfs_parse_fs_string, FsContext}, ida::Ida, namei::namei, ns_common::NsCommon, path::Path, super_block::vfs_get_tree};

static mut VFSMOUNT_HLIST : BTreeMap<*mut DEntry, *mut VFSMount> = BTreeMap::new();
static mut MOUNT_HLIST : BTreeMap<*mut DEntry, *mut Mountpoint> = BTreeMap::new();
static mut SYSCTL_MOUNT_MAX : u32 = u32::MAX;
static mut MNT_ID_IDA : Ida = Ida::new();
static mut MNT_GROUP_IDA : Ida = Ida::new();

const PATH_MAX : usize = 4096;

bitflags! {
    #[derive(Copy, Clone)]
    pub struct MntFlags : u32
    {
        const NOSUID = bit!(0);
        const NODEV = bit!(1);
        const NOEXEC = bit!(2);
        const NOATIME = bit!(3);
        const NODIRATIME = bit!(4);
        const RELATIME = bit!(5);
        const READONLY = bit!(6);
        const NOSYMFOLLOW = bit!(7);
        const SHRINKABLE = bit!(8);
        const WRITE_HOLD = bit!(9);
        const SHARED = 0x1000;
        const UNBINDABLE = 0x2000;
        const SHARED_MASK = MntFlags::UNBINDABLE.bits();
        const USER_SETTABLE_MASK = 0xff;
        const ATIME_MASK = 0x8 | 0x10 | 0x20;
        const INTERNEL = 0x400;
        const LOCK_ATIME = 0x40000;
        const LOCK_NOEXEC = 0x80000;
        const LOCK_NOSUID = 0x100000;
        const LOCK_NODEV = 0x200000;
        const LOCK_READONLY = 0x400000;
        const LOCKED = 0x800000;
        const DOOMED = 0x1000000;
        const SYNC_UMOUNT = 0x2000000;
        const MARKED = 0x4000000;
        const UMOUNT = 0x8000000;
        const ONRB = 0x10000000;
    } 
}

enum MntTreeFlag {
    Empty = 0,
    MntTreeMove = bit!(0),
    MntTreeBeneath = bit!(1)
}

struct MntNamespace
{
    ns : NsCommon,
    root : *mut Mount,
    ucounts : usize,
    nr_mounts : u32,
    pending_mounts : u32,
    seq : u64,
    rb_node : BTreeSet<*mut Mount>
}

impl MntNamespace
{
    pub const fn is_anon_ns(&self) -> bool
    {
        self.seq == 0
    }
    pub fn new() -> *mut Self
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()) as *mut Self;
            (*ptr) = Self
            {
                ns: NsCommon
                {
                    stashed: null_mut()
                }, 
                root: null_mut(),
                ucounts : 0,
                nr_mounts : 0,
                pending_mounts : 0,
                seq : 0,
                rb_node : BTreeSet::new()
            };
            ptr
        }
    }

    pub fn add_mount(&mut self, mnt : *mut Mount)
    {
        unsafe
        {
            self.rb_node.insert(mnt);
        }
    }
    
    #[inline(always)]
    pub fn get(&mut self)
    {
        self.ns.count.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    }
}

pub fn lookup_mnt(dentry : *mut DEntry) -> *mut VFSMount
{
    unsafe
    {
        match VFSMOUNT_HLIST.get(&dentry) {
            Some(mnt) => *mnt,
            None => null_mut(),
        }
    }
}

pub fn find_mount(dentry : *mut DEntry) -> *mut Mountpoint
{
    unsafe
    {
        match MOUNT_HLIST.get(&dentry) {
            Some(mnt) => *mnt,
            None => null_mut(),
        }
    }
}

pub fn sys_mount(dev_name : *const c_char, dir_name : *const c_char, fstype : *const c_char, flags : u32, data : *const c_void) -> Err
{
    unsafe
    {
        let sdev_name = match CStr::from_ptr(dev_name).to_str() {
            Ok(str) => {
                if str.len() == 0
                {
                    return -EFAULT;
                }
                if str.len() > PATH_MAX
                {
                    return -EINVAL;
                }
                Arc::new(String::from(str))
            },
            Result::Err(_) => return -EINVAL,
        };
        let sdir_name = match CStr::from_ptr(dir_name).to_str() {
            Ok(str) => 
            {
                if str.len() == 0
                {
                    return -EFAULT;
                }
                if str.len() > PATH_MAX
                {
                    return -EINVAL;
                }
                Arc::new(String::from(str))
            },
            Result::Err(_) => return -EINVAL,
        };
        let stype_name = match CStr::from_ptr(fstype).to_str() {
            Ok(str) => 
            {
                if str.len() == 0
                {
                    return -EFAULT;
                }
                if str.len() > PATH_MAX
                {
                    return -EINVAL;
                }
                Arc::new(String::from(str))
            },
            Result::Err(_) => return -EINVAL,
        };
        do_mount(sdev_name, sdir_name, stype_name, flags, data)
    }
}

pub fn path_mount(dev_name : Arc<String>, path : Path, type_name : Arc<String>, flags : u32, data : *const c_void) -> Err
{
    unsafe
    {
        let mnt_flags = MntFlags::empty();
        let mut sb_flags = 0;
        let mut ret = 0;
        if !data.is_null()
        {
            *(data.offset(PAGE_SIZE as isize - 1) as *mut i8) = 0;
        }
        sb_flags = flags & (SB_RDONLY | SB_SYNCHRONOUS | SB_MANDLOCK | SB_DIRSYNC | SB_SILENT | SB_POSIXACL | SB_LAZYTIME | SB_I_VERSION);
        do_new_mount(path, type_name, sb_flags, mnt_flags, dev_name, data)
        // MOUNT_HLIST.insert(dstination, (*mount).mnt_mp);
    }
}

pub fn do_mount(dev_name : Arc<String>, dir_name : Arc<String>, fstype : Arc<String>, flags : u32, data_page : *const c_void) -> Err
{
    unsafe
    {
        let mount_path = namei(dev_name.as_ptr() as *mut i8);
        let dev_path = namei(dir_name.as_ptr() as *mut i8);
        let mnt_sb = (*dev_path.dentry).d_inode as *mut LogicalPart; // todo!()
        let mount = find_mount(mount_path.dentry);
        // path_mount(mnt_sb, mount_path, real_mount(mount_path.mnt), dev_name)
        0
    }
}

fn vfs_create_mount(fc : *mut FsContext) -> *mut VFSMount
{
    unsafe
    {
        let mnt;
        if (*fc).root.is_null()
        {
            return err_ptr(-EINVAL);
        }
        mnt = Mount::new((*(*fc).root).d_sb);
        if mnt.is_null()
        {
            return err_ptr(-ENOMEM);
        }
        (*mnt).mnt.mnt_sb = (*(*fc).root).d_sb;
        (*(*mnt).mnt_mp).m_dentry = (*(*fc).root).dget();
        (*mnt).mnt.mnt_root = (*fc).root;
        (*mnt).mnt_parent = mnt;

        (*mnt).mnt_instance.head_insert(&mut (*(*mnt).mnt.mnt_sb).s_mounts);

        addr_of_mut!((*mnt).mnt)
    }
    
}

fn do_new_mount(path : Path, fstype : Arc<String>, sb_flags : u32, mnt_flags : MntFlags, name : Arc<String>, data : *const c_void) -> Err
{
    unsafe
    {
        let fs_type = FS.get_fs_type(name.clone());
        let fc = FsContext::context_for_mount(fs_type, sb_flags);
        let mut err = 0;
        if is_err(fc)
        {
            return ptr_err(fc);
        }
        if 0 != err && !name.is_empty()
        {
            err = vfs_parse_fs_string(fc, "source", &name);
        }
        if err != 0
        {
            err = vfs_get_tree(fc);
        }

        if 0 != err
        {
            err = do_new_mount_fc(fc, path, mnt_flags);
        }

        FsContext::puts_context(fc);
        err
    }
}

fn real_mount(vfsmount : *mut VFSMount) -> *mut Mount
{
    unsafe
    {
        container_of!(vfsmount, Mount, mnt)
    }
}

fn do_new_mount_fc(fc : *mut FsContext, mountpoint : Path, mut mnt_flags : MntFlags) -> Err
{
    unsafe
    {
        let sb = (*(*fc).root).d_sb;
        if mount_too_revealing(sb, &mut mnt_flags)
        {
            return -EPERM;
        }
        let mnt = vfs_create_mount(fc);
        if is_err(mnt)
        {
            return ptr_err(mnt);
        }
        let mp = get_mountpoint(mountpoint.dentry);
        do_add_mount(real_mount(mnt), mp, mountpoint, mnt_flags)
    }
}

fn graft_tree(mnt : *mut Mount, p : *mut Mount, mp : *mut Mountpoint) -> Err
{
    unsafe
    {
        if (*(*mnt).mnt.mnt_sb).s_flags & SB_NOUSER != 0
        {
            return -EINVAL;
        }
        if !(*(*mp).m_dentry).is_dir() != (*(*mnt).mnt.mnt_root).is_dir()
        {
            return -ENOTDIR;
        }
        attach_recursive_mnt(mnt, p, mp, MntTreeFlag::Empty)
    }
}

fn attach_recursive_mnt(source_mnt : *mut Mount, top_mnt : *mut Mount, mut dest_mp : *mut Mountpoint, flags : MntTreeFlag) -> Err
{
    unsafe
    {
        let moving;
        let beneath;
        let ns = (*top_mnt).mnt_ns;
        let mut err = 0;
        let mut dest_mnt;
        match flags {
            MntTreeFlag::MntTreeMove => 
            {
                moving = true;
                beneath = false;
            },
            MntTreeFlag::MntTreeBeneath => 
            {
                moving = false;
                beneath = true;
            },
            _ => 
            {
                moving = false;
                beneath = false;
            }
        }
        let smp = get_mountpoint((*source_mnt).mnt.mnt_root);
        if is_err(smp)
        {
            return ptr_err(smp);
        }
        if !moving
        {
            err = count_mounts(ns, source_mnt);
            if err != 0
            {
                (*ns).pending_mounts = 0;
                return err;
            }
        }
        if beneath
        {
            dest_mnt = (*top_mnt).mnt_parent;
        }
        else {
            dest_mnt = top_mnt;
        }
        if is_mnt_shared(dest_mnt)
        {
            err = invent_group_ids(source_mnt, true);
            if err != 0
            {
                (*ns).pending_mounts = 0;
                return err;
            }
            err = propagate_mnt(dest_mnt, dest_mp, source_mnt, &BTreeSet::<*mut Mount>::new());
        }
        if err != 0
        {
            todo!("clean mnt ids");
        }
        if is_mnt_shared(dest_mnt)
        {
            let mut p = source_mnt;
            while !p.is_null()
            {
                set_mnt_shared(&mut *p);
                p = (*p).next_mnt(source_mnt);
            }
        }
        if moving
        {
            if beneath
            {
                dest_mp = smp;
            }
            todo!();
        }
        else
        {
            if !(*source_mnt).mnt_ns.is_null()
            {
                todo!();
            }
            if beneath
            {
                todo!();
            }
            else
            {
                todo!();
            }
            commit_tree(source_mnt);
        }
        0
    }
}

fn propagate_mnt(dest_mnt : *mut Mount, dest_mp : *mut Mountpoint, source_mnt : *mut Mount, tree_list : &BTreeSet<*mut Mount>) -> Err
{
    unsafe
    {
        let mut last_dest = dest_mnt;
        let mut first_source = source_mnt;
        let mut last_source = source_mnt;
        let mut dest_master = (*dest_mnt).mnt_master;
        let mut ret = 0;
        let mut n = next_peer(dest_mnt);
        while n != dest_mnt
        {
            ret = propagate_one(n, dest_mp);
            if ret != 0
            {
                return ret;
            }
            n = next_peer(n);
        }

        let mut m = next_group(dest_mnt, dest_mnt);
        while !m.is_null() {
            n = m;
            loop {
                ret = propagate_one(n, dest_mp);
                if ret != 0
                {
                    return ret;
                }
                n = next_peer(n);
                if n != m
                {
                    break;
                }
            }
            m = next_group(m, dest_mnt)
        }
        for n in tree_list
        {
            m = (**n).mnt_parent;
            if (*m).mnt_master != (*dest_mnt).mnt_master
            {
                (*(*m).mnt_master).mnt.mnt_flags = (*(*m).mnt_master).mnt.mnt_flags.difference(MntFlags::MARKED); 
            }
        }
    }
    0
}

fn next_peer(p : *mut Mount) -> *mut Mount
{
    unsafe
    {
        container_of!((*p).mnt_share.next, Mount, mnt_share)
    }
}

fn next_group(mut m : *mut Mount, origin : *mut Mount) -> *mut Mount
{
    unsafe
    {
        loop {
            loop {
                let mut next;
                if is_mnt_new(m) && !(*m).mnt_slave_list.is_empty()
                {
                    return container_of!((*m).mnt_slave_list.next, Mount, mnt_slave);
                }
                next = next_peer(m);
                if (*m).mnt_group_id == (*origin).mnt_group_id
                {
                    if next == origin
                    {
                        return null_mut();
                    }
                }
                else if (*m).mnt_slave.next != addr_of_mut!((*next).mnt_slave) {
                    break;
                }
                m = next;
            }
            loop {
                let mut master = (*m).mnt_master;
                if (*m).mnt_slave.next != addr_of_mut!((*master).mnt_slave_list)
                {
                    return (*m).next_slave();
                }
                m = next_peer(master);
                if (*master).mnt_group_id == (*origin).mnt_group_id
                {
                    break;
                }
                if (*master).mnt_slave.next == addr_of_mut!((*m).mnt_slave)
                {
                    break;
                }
                m = master
            }
            if m == origin
            {
                return null_mut();
            }
        }
    }

}

#[inline(always)]
fn is_mnt_new(m : *mut Mount) -> bool
{
    unsafe
    {
        !(*m).mnt_ns.is_null() || (*(*m).mnt_ns).is_anon_ns()
    }
}

fn propagate_one(m : *mut Mount, dest_mp : *mut Mountpoint) -> Err
{
    unsafe
    {
        if is_mnt_new(m) //skip
        {
            return 0;
        }
        unimplemented!("now can only be called by propagate_mnt()");
    }
}

fn mnt_alloc_group_id(mnt : *mut Mount) -> Err
{
    unsafe
    {
        let res = MNT_GROUP_IDA.alloc_min(1);
        if res < 0
        {
            return res as i64;
        }
        (*mnt).mnt_group_id = res;
        0        
    }
}

pub fn invent_group_ids(mnt : *mut Mount, recures : bool) -> Err
{
    unsafe
    {
        let mut p = mnt;
        while !p.is_null()
        {
            if (*p).mnt_group_id == 0 && is_mnt_shared(p)
            {
                let err = mnt_alloc_group_id(p);
                return err;
            }
            p = if recures { (*p).next_mnt(mnt) } else { null_mut() };
        }
        0
    }
}

#[inline(always)]
fn is_mnt_shared(dest_mnt : *mut Mount) -> bool
{
    unsafe
    {
        (*dest_mnt).mnt.mnt_flags.contains(MntFlags::SHARED)
    }
}

fn count_mounts(ns : *mut MntNamespace, mnt : *mut Mount) -> Err
{
    unsafe
    {
        let mut max = SYSCTL_MOUNT_MAX;
        if (*ns).nr_mounts >= max // read_once!()
        {
            return -ENOSPC
        }
        max -= (*ns).nr_mounts;
        if (*ns).pending_mounts >= max
        {
            return -ENOSPC
        }
        let mut mounts = 0;
        let mut p = mnt;
        while !p.is_null() {
            mounts += 1;
            p = (*p).next_mnt(mnt);
        }
        if mounts > max
        {
            return -ENOSPC;
        }
        (*ns).pending_mounts += mounts;
        return 0
    }
}

#[inline(always)]
fn path_mounted(path : &Path) -> bool
{
    unsafe {
        (*path.mnt).mnt_root == path.dentry
    }
}

fn do_add_mount(newmount : *mut Mount, mp : *mut Mountpoint, path : Path, mnt_flags : MntFlags ) -> Err
{
    unsafe
    {
        let parent = real_mount(path.mnt);
        if (*path.mnt).mnt_sb == (*newmount).mnt.mnt_sb && path_mounted(&path)
        {
            return -EBUSY
        }
        if (*(*newmount).mnt.mnt_root).is_symlink()
        {
            return -EINVAL
        }
        (*newmount).mnt.mnt_flags = mnt_flags;
        graft_tree(newmount, parent, mp)
    }
}

fn mount_too_revealing(sb : *mut LogicalPart, new_mount_flags : &mut MntFlags) -> bool
{
    unsafe
    {
        let requited_flags = SB_I_NOEXEC + SB_I_NODEV;
        let s_iflags = (*sb).s_flags;
        if !((SB_I_USERNS_VISIBLE & s_iflags) != 0)
        {
            return false;
        }
        if (s_iflags & requited_flags) != requited_flags
        {
            return true;
        }
        return false;
    }
}

fn commit_tree(mnt : *mut Mount)
{
    unsafe
    {
        let parent = (*mnt).mnt_parent;
        // (*parent).
    }

}

fn get_mountpoint(dentry : *mut DEntry) -> *mut Mountpoint
{
    unsafe
    {
        let mp =  find_mount(dentry);
        let new;
        if !mp.is_null()
        {
            (*mp).m_count += 1;
            return mp;
        }
        new = Mountpoint::new(dentry);
        MOUNT_HLIST.insert(dentry, new);
        new
    }

}

pub struct Mountpoint
{
    pub m_dentry : *mut DEntry,
    pub m_count : u32
}

pub struct Mount
{
    pub mnt_parent : *mut Mount,
    pub mnt_mp : *mut Mountpoint,
    pub mnt_devname : Arc<String>,
    pub mnt : VFSMount,
    pub nmt_devname : Arc<String>,
    pub mnt_mounts : ListHead,
    pub mnt_share : ListHead,
    pub mnt_slave_list : ListHead,
    pub mnt_child : ListHead,
    pub mnt_instance : ListHead,
    pub mnt_slave : ListHead,
    pub mnt_ns : *mut MntNamespace,
    pub mnt_id : i32,
    pub mnt_group_id : i32,
    pub mnt_master : *mut Mount
}

pub struct VFSMount
{
    pub mnt_sb : *mut LogicalPart,
    pub mnt_root : *mut DEntry,
    pub mnt_flags : MntFlags
}

impl Mountpoint
{
    fn new(dentry : *mut DEntry) -> *mut Self
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()) as *mut Self;
            if ptr.is_null()
            {
                return err_ptr(-ENOMEM);
            }
            (*dentry).d_flags.insert(DEntryFlags::MOUNTED);
            (*ptr) = Self { m_dentry: dentry, m_count: 1 };
            ptr
        }
    }
}

impl Mount {
    fn new(mnt_sb : *mut LogicalPart) -> *mut Self
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()) as *mut Self;
            (*ptr) = Self { mnt_parent: null_mut(), mnt_mp: null_mut(), mnt_devname: Arc::new(String::new()), mnt: VFSMount { mnt_sb, mnt_root: null_mut(), mnt_flags: MntFlags::empty() }, nmt_devname: Arc::new(String::from("none")), mnt_mounts: ListHead::empty(), mnt_ns: null_mut(), mnt_id: 0, mnt_group_id: 0, mnt_master: null_mut(), mnt_share: ListHead::empty(), mnt_slave_list: ListHead::empty(), mnt_slave: ListHead::empty(), mnt_child: ListHead::empty(), mnt_instance: ListHead::empty() };
            ptr
        }
    }

    fn next_slave(&self) -> *mut Self
    {
        unsafe { container_of!(self.mnt_slave.next, Mount, mnt_slave) }
    }

    fn next_mnt(&mut self, root : *mut Mount) -> *mut Mount
    {
        unsafe
        {
            let mut p = addr_of_mut!(*self);
            let next = self.mnt_mounts.next.cast::<ListHead>();
            if next == addr_of_mut!((*p).mnt_mounts).cast::<ListHead>()
            {
                loop {
                    if p == root
                    {
                        return null_mut()
                    }
                    if next != addr_of_mut!((*(*p).mnt_parent).mnt_mounts)
                    {
                        break;
                    }
                    p = (*p).mnt_parent;
                }
            }
            container_of!(next, Mount, mnt_child)
        }

    }
}

pub fn vfs_kern_mount(_type : *mut FileSystemType, flags : u32, name : Arc<String>, data : *mut c_void) -> *mut VFSMount
{
    unsafe
    {
        let fc = FsContext::context_for_mount(_type, flags);
        let mut ret = 0;
        let mnt;
        if _type.is_null()
        {
            return err_ptr(-EINVAL);
        } 
        if (*name).len() != 0
        {
            ret = vfs_parse_fs_string(fc, "source", &name);
        } 
        if ret == 0
        {
            todo!()
        } 
        if ret == 0
        { 
            mnt = fc_mount(fc);
        } 
        else
        {
            mnt = err_ptr(ret);
        }
        FsContext::puts_context(fc);
        mnt
    }
}

pub fn fc_mount(fc : *mut FsContext) -> *mut VFSMount
{
    let err = vfs_get_tree(fc);
    if err == 0
    {
        return vfs_create_mount(fc);
    }
    err_ptr(err)
}

#[__init]
pub fn init_mount_tree()
{
    unsafe
    {
        let mnt = vfs_kern_mount(addr_of!(ROOTFS_FS_TYPE), 0, Arc::new(String::from("rootfs")), null_mut());
        if is_err(mnt)
        {
            panic!("Can't create rootfs");
        }
        let ns = MntNamespace::new();
        if is_err(ns)
        {
            panic!("Can't allocate initial namespace");
        }
        let m = real_mount(mnt);
        (*ns).root = m;
        (*ns).nr_mounts = 1;
        (*ns).add_mount(m);
        
    }
}
