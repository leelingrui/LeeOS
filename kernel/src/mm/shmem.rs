use core::mem::ManuallyDrop;
use core::{alloc::Layout, mem::size_of, ptr::null_mut, sync::atomic::AtomicI32};
use core::intrinsics::unlikely;

use alloc::alloc::alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use proc_macro::__init;

use crate::fs::dcache::DEntryOperations;
use crate::fs::file::{DirEntry, FSPermission};
use crate::kernel::time;
use crate::{fs::file::LogicalPart, kernel::{errno_base::ENOSPC, list::ListHead, process::{Gid, Uid}, semaphore::SpinLock, time::Time, Off}};

use super::{memory::MemoryPool, page::Pageflags, slub::{KMallocInfoStruct, KmemCache}};

pub static mut DEV_FS : *mut ShmemSbInfo = null_mut();
const BOGO_INODE_SIZE : i64 = 1024;
const VM_NORESERV : u32 = 0x00200000;
const F_SEAL_SEAL : u32 = 1;
static mut SHMEM_INODE_CACHEP : *mut KmemCache = null_mut();
pub const SHMEM_DIR_OPERATION : DEntryOperations = DEntryOperations
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
type Ino = u64;

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
        (*SHMEM_INODE_CACHEP) = KmemCache::create_cache("shmem_inode_cache", size_of::<KmemCache>() as u32, size_of::<KmemCache>() as u32, null_mut(), Pageflags::empty());
        (*SHMEM_INODE_CACHEP).link_to_cache_list();
        DEV_FS = ShmemSbInfo::new();
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
    // fn reserve_inode(&mut self, inoref : &mut Ino) -> i64
    // {
    //     self.stat_lock.acquire(1);
    //     if self.max_inodes > 0
    //     {
    //         if self.free_ispace >= BOGO_INODE_SIZE
    //         {
    //             self.free_ispace -= BOGO_INODE_SIZE;
    //         }
    //         else
    //         {
    //             self.stat_lock.release(1);
    //             return -ENOSPC;
    //         }
    //         let mut ino = self.next_ino;
    //         self.next_ino += 1;
    //         if unlikely(ino == 0)
    //         {
    //             ino = self.next_ino;
    //             ino += 1;
    //         }
    //         if unlikely(!self.full_inums && ino >= Ino::MAX)
    //         {
    //             panic!("inode number overfellow");
    //         }
    //         *inoref = ino;
    //     }
    //     self.stat_lock.release(1);
    //     return 0;
    // }

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
            sbi
        }
    }
}