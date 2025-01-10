use core::mem::ManuallyDrop;
use core::{alloc::Layout, mem::size_of, ptr::{null_mut, addr_of, addr_of_mut, null}, ffi::c_void, sync::atomic::AtomicI32};
use core::intrinsics::unlikely;

use alloc::alloc::alloc;
use alloc::collections::{BTreeSet, BTreeMap};
use alloc::string::String;
use proc_macro::__init;

use crate::fs::dcache::{DEntryOperations, DEntry};
use crate::fs::{file::{DirEntry, FSPermission, FileMode, FileFlag}, libfs::simple_lookup, super_block::{kill_litter_super, get_tree_nodev}, fs_context::{FsContextOperations, FsContext}};
use crate::{kernel::{time, Err}, printk};
use crate::{fs::{file::{LogicalPart, FSType, FS}, fs::{FileSystemType, SB_KERNMOUNT, FileSystemFlags}, mnt_idmapping::{MntIdmap, NOP_MNT_IDMAP}, inode::{Inode, InodeOperations}}, kernel::{{errno_base::{ENOSPC, ENOMEM, err_ptr, is_err, ptr_err}, io::SECTOR_SIZE}, list::ListHead, device::DevT, process::{Gid, Uid}, semaphore::SpinLock, time::Time, Off, sched::get_current_running_process}};

use super::{memory::{MemoryPool, PAGE_SIZE}, page::Pageflags, slub::{KMallocInfoStruct, KmemCache}};
pub static mut DEV_FS : *mut ShmemSbInfo = null_mut();
const BOGO_INODE_SIZE : i64 = 1024;
const VM_NORESERV : u32 = 0x00200000;
const F_SEAL_SEAL : u32 = 1;

static mut SHMEM_FS_TYPE : FileSystemType = FileSystemType
{
    name : "shmem\0",
    next : null_mut(),
    init_fs_context : Some(shmem_init_fs_context),
    fs_supers : BTreeMap::new(),
    kill_sb : Some(kill_litter_super),
    fs_flags : FileSystemFlags::from_bits_retain(FileSystemFlags::USERNS_MOUNT.bits() | FileSystemFlags::ALLOW_IDMAP.bits())
};

pub fn shmem_get_tree(fc : *mut FsContext) -> Err
{
    get_tree_nodev(fc, ShmemSbInfo::fill_super)
}

static mut SHMEM_FS_CONTEXT_OPS : FsContextOperations = FsContextOperations
{
    parse_param: None,
    get_tree: Some(shmem_get_tree),
    parse_monolithic: None
};

static mut SHMEM_INODE_CACHEP : *mut KmemCache = null_mut();
pub static SHMEM_DIR_OPERATION : DEntryOperations = DEntryOperations
{
    d_revalidate: None,
    d_weak_revalidate: None,
    d_hash: None,
    d_compare: None,
    d_delete: None,
    d_init: None,
    d_release: None,
    d_prune: None,
    d_iput: None,
    d_dname: None,
};

pub static SHMEM_SPECIAL_INODE_OPERATIONS : InodeOperations = InodeOperations
{
    lookup: None,
    mknod: None,
    mkdir: None
};

pub static SHMEM_DIR_INODEOPERATIONS : InodeOperations = InodeOperations
{
    lookup: Some(simple_lookup),
    mknod: Some(shmem_mknod),
    mkdir: Some(shmem_mkdir)
};

pub static SHMEM_INODE_OPERATIONS : InodeOperations = InodeOperations
{
    lookup: None,
    mknod: None,
    mkdir: None
};
type Ino = u64;

struct ShmemOptions
{
    blocks : usize,
    inodes : usize,
    gid : Gid,
    uid : Uid,
    noswap : bool,
    mode : FileMode,
    quota_types : u16,
    qlimits : ShmemQuotaLimits
}

impl ShmemOptions
{
    fn new() -> *mut Self
    {
        unsafe
        {
           alloc::alloc::alloc_zeroed(Layout::new::<Self>()) as *mut Self
        }
    }
}
pub fn shmem_init_fs_context(fs_context : *mut FsContext) -> Err
{
    unsafe
    {
        let ctx = ShmemOptions::new();
        let pcb = get_current_running_process();
        (*ctx).mode = FileMode::from_bits_truncate(0o777); //.insert(FileMode::S_ISVTX);
        (*ctx).uid = (*pcb).uid;
        (*ctx).gid = (*pcb).gid;
        (*fs_context).fs_private = ctx as *mut c_void;
        (*fs_context).ops = addr_of_mut!(SHMEM_FS_CONTEXT_OPS);
        0
    }
}
pub struct ShmemQuotaLimits {
	usrquota_bhardlimit : usize, /* Default user quota block hard limit */
	usrquota_ihardlimit : usize, /* Default user quota inode hard limit */
	grpquota_bhardlimit : usize, /* Default group quota block hard limit */
	grpquota_ihardlimit : usize /* Default group quota inode hard limit */
}

impl ShmemQuotaLimits {
    pub fn empty() -> Self
    {
        Self { usrquota_bhardlimit: 0, usrquota_ihardlimit: 0, grpquota_bhardlimit: 0, grpquota_ihardlimit: 0 }
    }
}

pub struct ShmemSbInfo
{
    max_blocks : usize,
    used_blocks : i64,
    max_inodes : usize,
    free_ispace : i64,
    stat_lock : SpinLock,
    mode : u32,
    uid : Uid,
    gid : Gid,
    full_inums : bool,
    noswap : bool,
    next_ino : Ino,
    shrinklist_lock : SpinLock,
    shrinklist : ListHead,
    shrinklist_len : usize,
    qlimits : ShmemQuotaLimits
}


#[__init]
pub fn init_shmem()
{
    unsafe
    {
        SHMEM_INODE_CACHEP = alloc(Layout::new::<KmemCache>()) as *mut KmemCache;
        (*SHMEM_INODE_CACHEP) = KmemCache::create_cache("shmem_inode_cache\0", size_of::<KmemCache>() as u32, size_of::<KmemCache>() as u32, null_mut(), Pageflags::empty());
        (*SHMEM_INODE_CACHEP).link_to_cache_list();
        DEV_FS = ShmemSbInfo::new();
        FS.register_filesystem(addr_of_mut!(SHMEM_FS_TYPE));
    }
}

union InodeInternalInfo
{
    dir_offsets : core::mem::ManuallyDrop<BTreeMap<String, DirEntry>>,
    shrinklist : ListHead,
    swaplist : ListHead
}

impl InodeInternalInfo
{
    fn new_shrinklist() -> Self
    {
        Self {
            shrinklist : ListHead::empty()
        }
    }

    fn new_swaplist() -> Self
    {
        Self {
            swaplist : ListHead::empty()
        }
    }

    fn new_dirinfo() -> Self
    {
        Self
        {
            dir_offsets : core::mem::ManuallyDrop::<BTreeMap::<String, DirEntry>>::new(BTreeMap::<String, DirEntry>::new())
        }
    }
}


pub struct ShmemInodeInfo
{
    lock : SpinLock,
    seals : u32,
    flags : u32,
    alloced : u32,
    swapped : u32,
    internal_info : InodeInternalInfo,
    i_crtime : Time,
    fallocend : Off,
    fsflags : u32,
    stop_eviction : AtomicI32,
}


impl ShmemSbInfo
{
    pub fn fill_super(lp : *mut LogicalPart, fc : *mut FsContext) -> Err
    {
        unsafe
        {
            // let ctx = (*fc).fs_private as *mut ShmemSbInfo;
            let sbi;
            sbi = ShmemSbInfo::new();
            if sbi.is_null()
            {
                return -ENOMEM;
            }
            (*lp).s_sbi = sbi.cast();
            // (*sbi).max_blocks = (*ctx).max_blocks;
            // (*sbi).max_inodes = (*ctx).max_inodes;
            (*sbi).free_ispace = (*sbi).max_inodes as i64 * BOGO_INODE_SIZE;
            if ((*lp).s_flags & SB_KERNMOUNT) != 0
            {
                // (*sbi).
                // a}
            }
            // (*sbi).uid = (*ctx).uid;
            // (*sbi).gid = (*ctx).gid;
            // (*sbi).full_inums = (*ctx).full_inums;
            // (*sbi).mode = (*ctx).mode;
        
            (*lp).logic_block_size = PAGE_SIZE as i32 / SECTOR_SIZE as i32;
            (*lp).old_fs_type = FSType::Shmem;
            (*lp).inode_count = (*sbi).max_inodes;


            let inode = shmem_get_inode(addr_of_mut!(NOP_MNT_IDMAP), lp, null_mut(), FileMode::from_bits_truncate((*sbi).mode as u16) | FileMode::IFDIR, 0, FileFlag::empty()/*FileFlag::VM_NORESERV*/);
            (*inode).i_uid = (*sbi).uid;
            (*inode).i_gid = (*sbi).gid;
            (*lp).s_root = DEntry::make_root(inode);
            if !(*lp).s_root.is_null()
            {
                return 0;
            }
            todo!();
        }
    }
    fn reserve_inode(&mut self, inoref : &mut Ino) -> i64
    {
        self.stat_lock.acquire(1);
        if self.max_inodes > 0
        {
            if self.free_ispace >= BOGO_INODE_SIZE
            {
                self.free_ispace -= BOGO_INODE_SIZE;
            }
            else
            {
                self.stat_lock.release(1);
                return -ENOSPC;
            }
            let mut ino = self.next_ino;
            self.next_ino += 1;
            if unlikely(ino == 0)
            {
                ino = self.next_ino;
                ino += 1;
            }
            if unlikely(!self.full_inums && ino >= Ino::MAX)
            {
                panic!("inode number overfellow");
            }
            *inoref = ino;
        }
         self.stat_lock.release(1);
        return 0;
    }

    pub fn mknod(&mut self, flag : u32) -> *mut ShmemInodeInfo
    {
        self.shmem_alloc_inode(flag)
    }

    pub fn mkdir(&mut self, flag : u32) -> *mut ShmemInodeInfo
    {
        unsafe
        {
            let new_dir_inode = self.shmem_alloc_inode(flag);
            (*new_dir_inode).internal_info = InodeInternalInfo::new_dirinfo();
            new_dir_inode
        }

    }

    pub fn shmem_alloc_inode(&mut self, flag : u32) -> *mut ShmemInodeInfo
    {
        unsafe {
            let new_inode = (*SHMEM_INODE_CACHEP).alloc() as *mut ShmemInodeInfo;
            (*new_inode).seals = F_SEAL_SEAL;
            (*new_inode).flags = flag & VM_NORESERV;
            (*new_inode).i_crtime = time::sys_time();
            new_inode
        }
    }

    fn default_max_blocks() -> usize
    {
        MemoryPool::total_pages() / 2
    }

    fn default_max_inodes() -> usize
    {
        MemoryPool::total_pages() / 2
    }

    pub fn new() -> *mut Self
    {
        unsafe
        {
            let sbi = alloc(Layout::new::<Self>()) as *mut Self;
            (*sbi) = Self { max_blocks: Self::default_max_blocks(), used_blocks: 0, max_inodes: Self::default_max_inodes(), free_ispace: Self::default_max_inodes() as i64 * 1024, stat_lock: SpinLock::new(1), mode: 0, uid: 0, gid: 0, full_inums: false, noswap: true, next_ino: 0, shrinklist_lock: SpinLock::new(1), shrinklist: ListHead::empty(), shrinklist_len: 0, qlimits: ShmemQuotaLimits::empty() };
            (*sbi).shrinklist.init();
            sbi
        }
    }
}

fn __shmem_get_inode(idmap : *mut MntIdmap, lp : *mut LogicalPart, dir : *mut Inode, mode : FileMode, dev : DevT, flags : FileFlag) -> *mut Inode
{
    unsafe
    {
        let inode;
        let sbi = (*lp).s_sbi as *mut ShmemSbInfo;
        inode = Inode::new(null(), FSPermission::all());
        (*inode).i_mode = mode.clone();
        if (inode.is_null())
        {
            return err_ptr(-ENOSPC);
         }
        let mut ino = 0;
        let info = (*sbi).shmem_alloc_inode(0);
        (*info).flags = flags.bits() as u32 & VM_NORESERV;
        if (is_err(info))
        {
            return info.cast();
        }
        // (*sbi).reserve_inode(ino);
        // (*inode).i_ino = ino;
        // (*inode).i_blocks = 0;
        (*inode).logical_part_ptr = lp;
        (*inode).inode_desc_ptr = info.cast();
        match mode & FileMode::IFMT
         {
            FileMode::IFREG => 
            {
                (*inode).i_operations = addr_of!(SHMEM_INODE_OPERATIONS);
            },
            FileMode::IFDIR => 
            {
                (*inode).i_operations = addr_of!(SHMEM_DIR_INODEOPERATIONS);
            },
            val => 
            {
                (*inode).i_operations = addr_of!(SHMEM_SPECIAL_INODE_OPERATIONS);
                (*inode).init_special_inode(val, dev);
             },
        }
        return inode;
    } 
}
#[inline(always)]
fn shmem_get_inode(idmap : *mut MntIdmap, lp : *mut LogicalPart, dir : *mut Inode, mode : FileMode, dev : DevT, flags : FileFlag) -> *mut Inode
{
    __shmem_get_inode(idmap, lp, dir, mode, dev, flags)
}

pub fn shmem_mknod(idmap : *mut MntIdmap, dir : *mut Inode, dentry : *mut DEntry, mode : FileMode, dev : DevT) -> Err
{
    unsafe
    {
        let inode;
        inode = shmem_get_inode(idmap, (*dir).logical_part_ptr, dir, mode, dev, FileFlag::empty() /* VM_NORESERV */);
        if is_err(inode) { return ptr_err(inode); }
        (*dentry).d_inode = inode;
        (*dentry).dget();
        return 0;
    }
}

pub fn shmem_mkdir(idmap : *mut MntIdmap, dir : *mut Inode, dentry : *mut DEntry, mode : FileMode) -> Err
{
    unsafe
    {
        let error = shmem_mknod(idmap, dir, dentry, mode | FileMode::IFDIR, 0);
        if error != 0 {
            return error;
        }
        (*dir).inc_nlink();
        0
    }
}

