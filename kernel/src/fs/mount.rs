use core::{alloc::Layout, ffi::{c_char, c_void, CStr}, ptr::{addr_of, addr_of_mut, null_mut}, sync::atomic::AtomicI64, intrinsics::{likely, unlikely}};
use proc_macro::__init; 
use alloc::{collections::{BTreeMap, BTreeSet, LinkedList}, string::String, sync::Arc};
use bitflags::bitflags;
use crate::{bit, container_of, kernel::{sched::get_current_running_process, semaphore::Semaphore, errno_base::{EEXIST, err_ptr, is_err, ptr_err, EBUSY, EFAULT, EINVAL, EISDIR, ENOMEM, ENOSPC, ENOTDIR, EPERM}, list::ListHead, Err}, fs::{pnode::set_mnt_shared, fs::{SB_RDONLY, SB_SYNCHRONOUS, SB_MANDLOCK, SB_DIRSYNC, SB_SILENT, SB_POSIXACL, SB_LAZYTIME, SB_I_VERSION, FileSystemType}}};
use crate::mm::memory::PAGE_SIZE;
use super::{dcache::{DEntry, DEntryFlags}, file::{LogicalPart, FS, ROOTFS_FS_TYPE}, fs::{SB_I_NODEV, SB_I_NOEXEC, SB_I_USERNS_VISIBLE, SB_NOUSER}, fs_context::{vfs_parse_fs_string, FsContext, parse_monolithic_mount_data}, ida::Ida, namei::namei, ns_common::NsCommon, path::Path, super_block::vfs_get_tree};

static mut VFSMOUNT_HLIST : BTreeMap<*mut DEntry, *mut VFSMount> = BTreeMap::new();
static mut MOUNT_HLIST : BTreeMap<*mut DEntry, *mut Mountpoint> = BTreeMap::new();
static mut SYSCTL_MOUNT_MAX : u32 = u32::MAX;
static mut MNT_ID_IDA : Ida = Ida::new();
static mut MNT_GROUP_IDA : Ida = Ida::new();
static mut MOUNT_LOCK : Semaphore = Semaphore::new(1);

const PATH_MAX : usize = 4096;
pub const ROOT_MOUNTFLAGS : u32 = SB_SILENT;
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

pub const MS_RDONLY : u32 = 1;  /* Mount read-only */
pub const MS_NOSUID : u32 = 2;  /* Ignore suid and sgid bits */
pub const MS_NODEV : u32 = 4;  /* Disallow access to device special files */
pub const MS_NOEXEC : u32 = 8;  /* Disallow program execution */
pub const MS_SYNCHRONOUS : u32 = 16;  /* Writes are synced at once */
pub const MS_REMOUNT : u32 = 32;  /* Alter flags of a mounted FS */
pub const MS_MANDLOCK : u32 = 64;  /* Allow mandatory locks on an FS */
pub const MS_DIRSYNC : u32 = 128; /* Directory modifications are synchronous */
pub const MS_NOSYMFOLLOW : u32 = 256; /* Do not follow symlinks */
pub const MS_NOATIME : u32 = 1024;    /* Do not update access times. */
pub const MS_NODIRATIME : u32 = 2048;    /* Do not update directory access times */
pub const MS_BIND : u32 = 4096;
pub const MS_MOVE : u32 = 8192;
pub const MS_REC : u32 = 16384;
pub const MS_VERBOSE : u32 = 32768;   /* War is peace. Verbosity is silence.MS_VERBOSE is deprecated. */
pub const MS_SILENT : u32 = 32768;
pub const MS_POSIXACL : u32 = (1<<16); /* VFS does not apply the umask */
pub const MS_UNBINDABLE : u32 = (1<<17); /* change to unbindable */
pub const MS_PRIVATE : u32 = (1<<18); /* change to private */
pub const MS_SLAVE : u32 = (1<<19); /* change to slave */
pub const MS_SHARED : u32 = (1<<20); /* change to shared */
pub const MS_RELATIME : u32 = (1<<21); /* Update atime relative to mtime/ctime. */
pub const MS_KERNMOUNT : u32 = (1<<22); /* this is a kern_mount call */
pub const MS_I_VERSION : u32 = (1<<23); /* Update inode I_version field */
pub const MS_STRICTATIME : u32 = (1<<24); /* Always perform atime updates */
pub const MS_LAZYTIME : u32 = (1<<25); /* Update the on-disk [acm]times lazily */

/* These sb flags are internal to the kernel */
pub const MS_SUBMOUNT : u32 = (1<<26);
pub const MS_NOREMOTELOCK : u32 = (1<<27);
pub const MS_NOSEC : u32 = (1<<28);
pub const MS_BORN : u32 = (1<<29);
pub const MS_ACTIVE : u32 = (1<<30);
pub const MS_NOUSER : u32 = (1<<31);

/*
 *  * Superblock flags that can be altered by MS_REMOUNT
 *   */
pub const MS_RMT_MASK : u32 =  (MS_RDONLY|MS_SYNCHRONOUS|MS_MANDLOCK|MS_I_VERSION|
                 MS_LAZYTIME);

/*
 *  * Old magic mount flag and mask
 *   */
pub const MS_MGC_VAL : u32 = 0xC0ED0000;
pub const MS_MGC_MSK : u32 = 0xffff0000;


/*
 *  * move_mount() flags.
 *   */
pub const MOVE_MOUNT_F_SYMLINKS : u32 = 0x00000001; /* Follow symlinks on from path */
pub const MOVE_MOUNT_F_AUTOMOUNTS : u32 = 0x00000002; /* Follow automounts on from path */
pub const MOVE_MOUNT_F_EMPTY_PATH : u32 = 0x00000004; /* Empty from path permitted */
pub const MOVE_MOUNT_T_SYMLINKS : u32 = 0x00000010; /* Follow symlinks on to path */
pub const MOVE_MOUNT_T_AUTOMOUNTS : u32 = 0x00000020; /* Follow automounts on to path */
pub const MOVE_MOUNT_T_EMPTY_PATH : u32 = 0x00000040; /* Empty to path permitted */
pub const MOVE_MOUNT_SET_GROUP : u32 = 0x00000100; /* Set sharing group instead */
pub const MOVE_MOUNT_BENEATH : u32 = 0x00000200; /* Mount beneath top mount */
pub const MOVE_MOUNT_MASK : u32 = 0x00000377;

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
            ptr.write(Self
            {
                ns: NsCommon
                {
                    stashed: null_mut(),
                    count: AtomicI64::new(0)
                }, 
                root: null_mut(),
                ucounts : 0,
                nr_mounts : 0,
                pending_mounts : 0,
                seq : 0,
                rb_node : BTreeSet::new()
            });
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
        let name_len = compiler_builtins::mem::strlen(dev_name);
        let sdev_name = Arc::new(String::from_raw_parts(dev_name.cast_mut().cast(), name_len, PAGE_SIZE).clone());
        let dir_len = compiler_builtins::mem::strlen(dir_name);
        let sdir_name = Arc::new(String::from_raw_parts(dir_name.cast_mut().cast(), dir_len, PAGE_SIZE).clone());
        let stype_name = if fstype.is_null()
        { 
            Arc::new(String::new())
        }
        else
        {
            let fstype_len = compiler_builtins::mem::strlen(fstype);
            Arc::new(String::from_raw_parts(fstype.cast_mut().cast(), fstype_len, PAGE_SIZE).clone())
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
        if (flags & MS_MOVE) != 0
        {
            return do_move_mount_old(path, dev_name);
        }
        do_new_mount(path, type_name, sb_flags, mnt_flags, dev_name, data)
        // MOUNT_HLIST.insert(dstination, (*mount).mnt_mp);
    }
}

pub fn do_mount(dev_name : Arc<String>, dir_name : Arc<String>, fstype : Arc<String>, flags : u32, data_page : *const c_void) -> Err
{
    unsafe
    {
        let mount_path = namei(dev_name.as_ptr() as *mut i8);
        path_mount(dev_name, mount_path, fstype, flags, data_page)
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
        let fs_type = FS.get_fs_type(fstype.clone());
        let fc = FsContext::context_for_mount(fs_type, sb_flags);
        let mut err = 0;
        if is_err(fc)
        {
            return ptr_err(fc);
        }
        if 0 == err && !name.is_empty()
        {
            err = vfs_parse_fs_string(fc, "source", &name);
        }
        if err == 0
        {
            err = vfs_get_tree(fc);
        }

        if 0 == err
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
        if (*(*mp).m_dentry).is_dir() != (*(*mnt).mnt.mnt_root).is_dir()
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
                mnt_set_mountpoint(dest_mnt, dest_mp, source_mnt);
            }
            commit_tree(source_mnt);
        }
        0
    }
}

fn mnt_set_mountpoint(mnt : *mut Mount, mp : *mut Mountpoint, child_mnt : *mut Mount)
{
    unsafe
    {
        (*mp).m_count += 1;
        (*mnt).mnt_count.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        (*child_mnt).mnt_mp = mp;
        (*child_mnt).mnt_parent = mnt;
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
        let n = (*parent).mnt_ns;
        // (*mnt).mnt_list
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
    pub mnt_master : *mut Mount,
    pub mnt_count : AtomicI64
}

pub struct VFSMount
{
    pub mnt_sb : *mut LogicalPart,
    pub mnt_root : *mut DEntry,
    pub mnt_flags : MntFlags
}

impl VFSMount
{
    pub fn mntget(&mut self)
    {
        unsafe
        {
            if addr_of!(self).is_null()
            {
                (*real_mount(addr_of_mut!(*self))).mnt_count.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    pub fn mntput(&mut self)
    {
        unsafe
        {
            if addr_of!(self).is_null()
            {
                (*real_mount(addr_of_mut!(*self))).mnt_count.fetch_sub(1, core::sync::atomic::Ordering::Relaxed);
            }
         }
    }

    #[inline(always)]
    fn is_mounted(&mut self) -> bool
    {
        unsafe
        {
            let ptr = (*real_mount(addr_of_mut!(*self))).mnt_ns;
            is_err(ptr) || ptr.is_null()
        }
    }
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
            ptr.write(Self { m_dentry: dentry, m_count: 1 });
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
            ptr.write(Self { mnt_parent: null_mut(), mnt_mp: null_mut(), mnt_devname: Arc::new(String::new()), mnt: VFSMount { mnt_sb, mnt_root: null_mut(), mnt_flags: MntFlags::empty() }, nmt_devname: Arc::new(String::from("none")), mnt_mounts: ListHead::empty(), mnt_ns: null_mut(), mnt_id: 0, mnt_group_id: 0, mnt_master: null_mut(), mnt_share: ListHead::empty(), mnt_slave_list: ListHead::empty(), mnt_slave: ListHead::empty(), mnt_child: ListHead::empty(), mnt_instance: ListHead::empty(), mnt_count: AtomicI64::new(0) });
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
            if !next.is_null()
            {
                container_of!(next, Mount, mnt_child)
            }
            else
            {
                null_mut()
            }
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
            parse_monolithic_mount_data(fc, data);
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
        let mnt = vfs_kern_mount(addr_of_mut!(ROOTFS_FS_TYPE), 0, Arc::new(String::from("rootfs")), null_mut());
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
        
        (*mnt).mnt_flags.insert(MntFlags::LOCKED);
        let mut root = Path::empty();
        root.mnt = mnt;
        root.dentry = (*mnt).mnt_root;
        let pcb = get_current_running_process();
        (*pcb).set_ipwd(&root);
        (*pcb).set_iroot(&root)
        // todo_ mnt_ns_add_tree()
    }
}

#[__init]
pub fn mount_root_generic(name : *const c_char, pretty_name : *const c_char, flags : u32) -> Err
{
    unsafe
    {
        let mut p = alloc::alloc::alloc(Layout::new::<[c_void; PAGE_SIZE]>());
        let p_start = p;
        let num_fs = FS.list_bdev_fs_names(p.cast(), PAGE_SIZE);
        let mut err = 0;
        let mut i = 0;
        while i < num_fs
        {
            err = do_mount_root(name, p.cast(), flags, null_mut());
            if err == 0
            {
                alloc::alloc::dealloc(p_start, Layout::new::<[c_void; PAGE_SIZE]>());
                return 0;
            }
            let len = compiler_builtins::mem::strlen(p.cast());
            p = p.offset(len as isize + 1);
            i += 1;
        }
        alloc::alloc::dealloc(p_start, Layout::new::<[c_void; PAGE_SIZE]>());
        panic!("VFS: Unable to mount root fs on {}", CStr::from_ptr(name).to_str().unwrap());
        err
    }
}

#[__init]
pub fn do_mount_root(name : *const c_char, fs : *const c_char, flags : u32, data : *const c_void) -> Err
{
    unsafe
    {
        let mut data_page = null_mut();
        if !data.is_null()
        {
            data_page = alloc::alloc::alloc(Layout::new::<[c_void; PAGE_SIZE]>()) as *mut c_void;
            if data_page.is_null()
            {
                return -ENOMEM;
            }
            compiler_builtins::mem::memcpy(data_page.cast(), data.cast(), PAGE_SIZE);
        }
        let ret = init_mount(name, "/root\0".as_ptr().cast(), fs, flags, data_page);
        if ret != 0
        {
            alloc::alloc::dealloc(data_page.cast(), Layout::new::<[c_void; PAGE_SIZE]>());
            return ret;
        }
        init_chdir("/root\0".as_ptr().cast());
        alloc::alloc::dealloc(data_page.cast(), Layout::new::<[c_void; PAGE_SIZE]>());
        ret
    }
}

#[__init]
pub fn init_chdir(filename : *const c_char) -> Err
{
    unsafe
    {
        let path = namei(filename);
        let pcb = get_current_running_process();
        (*pcb).set_ipwd(&path);
        0
    }
}

#[__init]
fn init_mount(dev_name : *const c_char, dir_name : *const c_char, type_page : *const c_char, flags : u32, data_page : *const c_void) -> Err
{
    unsafe
    {
        sys_mount(dev_name, dir_name, type_page, flags, data_page)
    }
}


fn do_move_mount_old(path : Path, old_name : Arc<String>) -> Err
{
    let old_path = namei(old_name.as_ptr().cast());
    if path.dentry.is_null()
    {
        return -EEXIST;
    }
    do_move_mount(old_path, path, false)
}

fn do_move_mount(old_path : Path, new_path : Path, beneath : bool) -> Err
{
    unsafe
    {
        let old = real_mount(new_path.mnt);
        let parent = (*old).mnt_parent;
        let mp = do_lock_mount(old_path, beneath);
        0
    } 
}

fn do_lock_mount(mut path : Path, beneath : bool) -> *mut Mountpoint
{
    unsafe
    {
        let mut mnt = path.mnt;
        let mp = err_ptr(-ENOMEM);
        let mut dentry = null_mut();
        loop
        {
            let m = real_mount(mnt);
            if beneath
            {
                todo!();
            }
            else
            {
                dentry = path.dentry;
            }
            // (*(*dentry).d_inode).lock() todo!
            if unlikely((*dentry).cant_mount())
            {
                // inode_unlock todo!
                return mp;
            }
            // namespace_lock todo!
            if beneath && (!(*mnt).is_mounted())
            {
                // namespace_unlock 
                // inode_unlock
                return mp;
            }
            mnt = lookup_mnt(path.dentry);
            if likely(mnt.is_null())
            {
                break;
            }
            // namespace_unlock
            // inode_unlock
            if beneath
            {
                (*dentry).dput();
            }
            path.mnt = mnt;
            path.dentry = (*(*mnt).mnt_root).dget();
        }
        null_mut()
    }
}
