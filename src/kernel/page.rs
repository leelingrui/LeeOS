use core::{ffi::c_void, sync::atomic::{self, AtomicI32}, mem::size_of};
use bitflags::bitflags;
use super::{list::{ListHead}, memory};
pub const NODES_WIDTH : u32 = 22;
pub const ZONES_WIDTH : u32 = size_of::<Pageflags>() as u32 * 8 - NODES_WIDTH;
pub const NODES_PGSHIFT : u32 = NODES_WIDTH;
pub const ZONES_PGSHIFT : u32 = ZONES_WIDTH;
pub const ZONES_MASK : u32 = 1 << ZONES_PGSHIFT - 1;
pub const NODES_MASK : u32 = 1 << NODES_PGSHIFT - 1;

bitflags!{
    #[derive(Clone, Copy)]
    pub struct Pageflags : u32
    {
        const PgLocked = 1 << 0;              /* Page is locked. Don't touch. */
        const PgError = 1 << 1;
        const PgReferenced = 1 << 2;
        const PgUptodate = 1 << 3;
        const PgDirty = 1 << 4;
        const PgLru = 1 << 5;
        const PgActive = 1 << 6;
        const PgSlab = 1 << 7;
        const PgOwnerPriv1 = 1 << 8;      /* Owner use. If pagecache, fs may use*/
        const PgArch1 = 1 << 9;
        const PgReserved = 1 << 10;
        const PgPrivate = 1 << 11;             /* If pagecache, has fs-private data */
        const PgPrivate2 = 1 << 12;           /* If pagecache, has fs aux data */
        const PgWriteback = 1 << 13;           /* Page is under writeback */
        const PgHead = 1 << 14;                /* A head page */
        const PgTail = 1 << 15;                /* A tail page */
        const PgSwapcache = 1 << 16;           /* Swap page: swp_entry_t in private */
        const PgMappedtodisk = 1 << 17;        /* Has blocks allocated on-disk */
        const PgReclaim = 1 << 18;             /* To be reclaimed asap */
        const PgSwapbacked = 1 << 19;          /* Page is backed by RAM/swap */
        const PgUnevictable = 1 << 20;         /* Page is "unevictable"  */
        const PgMlocked = 1 << 21;             /* Page is vma mlocked */
        const PgUncached = 1 << 22;            /* Page has been mapped as uncached */
        const PgHwpoison = 1 << 23;            /* hardware poisoned page. Don't touch */
        const PgCompoundLock = 1 << 24;
        const PgChecked = 1 << 8;
        const PgFsCache = 1 << 12;
        const PgPinned = 1 << 8;
        const PgSavePinned = 1 << 4;
        const PgSlobFree = 1 << 11;
    }
}



bitflags! {
    #[derive(Clone, Copy)]
    pub struct GFP : u32
    {
        const __DMA = 0x01;
        const __HIGHMEM =  0x02;
        const __DMA32 = 0x04;
        const __MOVABLE = 0x08;
        const __RECLAIMABLE = 0x10;
        const __HARDWALL = 0x20;
        const __THISNODE = 0x40;
        const __ACCOUNT = 0x80;
        const __HIGH = 0x100;
        const __ATOMIC = 0x200;
        const __MEMALLOC = 0x400;
        const __NOMEMALLOC = 0x800;
        const __IO = 0x1000;
        const __FS = 0x2000;
        const __DIRECT_RECLAIM = 0x4000;
        const __KSWAPD_RECLAIM = 0x8000;
        const __RECLAIM = 0x10000;
        const __REPEAT = 0x20000;
        const __NOFAIL = 0x40000;
        const __NORETRY = 0x80000;
        const __COLD = 0x100000;
        const __NOWARN = 0x200000;
        const __ZERO = 0x400000;
        const __NOTRACK = 0x800000;
        const __OTHER_NODE = 0x100000;
        const __WAIT = 0x200000;
        const ATOMIC = Self::__HIGH.bits();
        const KERNEL = Self::__WAIT.bits() | Self::__IO.bits() | Self::__FS.bits();
        const NOIO = Self::__WAIT.bits();
        const NOFS = Self::__WAIT.bits() | Self::__IO.bits();
        const USER = Self::__WAIT.bits() | Self::__IO.bits() | Self::__FS.bits() | Self::__HARDWALL.bits();
        const IOFS = Self::__IO.bits() | Self::__FS.bits();
    }
}

#[repr(C)]
pub struct Page
{
    pub flags : Pageflags,
    pub lru : ListHead,
    pub filter : *mut c_void,
    pub reserved : u64,
    pub _refcount : AtomicI32
}

impl Page {

}