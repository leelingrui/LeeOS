use core::{alloc::Layout, ffi::c_char, mem::ManuallyDrop, ptr::{self, addr_of_mut, null_mut}, sync::atomic::AtomicI64};
use alloc::{collections::BTreeMap, string::{String, ToString}};

use crate::kernel::semaphore::RWLock;

use super::{file::{FileMode, LogicalPart, FS}, inode::Inode};


pub type RevalidateFunc = fn(*mut DEntry, u32) -> i64;
pub type HashFunc = fn(&DEntry, &QStr) -> i64;
pub type CompareFunc = fn(&DEntry, u32, &c_char, &QStr) -> i64;
pub type DeleteFunc = fn(&mut DEntry) -> i64;
pub type InitFunc = fn(&mut DEntry) -> i64;
pub type ReleaseFunc = fn(&mut DEntry) -> i64;
pub type PruneFunc = fn(&mut DEntry) -> i64;
pub type InodePutFunc = fn(&mut DEntry, *mut Inode) -> i64;
pub type NameFunc = fn(&mut DEntry, *const c_char, u32) -> i64;
// pub type ManageFunc = fn(&mut DEntry, )

pub struct DEntryOperations
{
    pub d_revalidate : Option<RevalidateFunc>,
    pub d_weak_revalidate : Option<RevalidateFunc>,
    pub d_hash : Option<HashFunc>,
    pub d_compare : Option<DeleteFunc>,
    pub d_delete : Option<DeleteFunc>,
    pub d_init : Option<InitFunc>,
    pub d_release : Option<ReleaseFunc>,
    pub d_prune : Option<PruneFunc>,
    pub d_iput : Option<InodePutFunc>,
    pub d_dname : Option<NameFunc>
}

impl DEntryOperations {
    pub fn empty() -> Self
    {
        Self
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
        }
    }
}

pub struct QStr
{
    name : String,
    hash : u64
}

bitflags::bitflags! {
    pub struct DEntryFlags : u32
    {
        const OP_HASH = 0x1;
        const OP_COMPARE = 0x2;
        const OP_REVALIDATE = 0x4;
        const OP_DELETE = 0x8;
        const OP_PRUNE = 0x10;
        const DISCONNECTED = 0x20;
        const REFERENED = 0x40;
        const DONTCACHE = 0x80;
        const CANT_MOUNT = 0x100;
        const GENOCIDE = 0x200;
        const SHRINK_LIST = 0x400;
        const OP_WEAK_REVALIDATE = 0x800;
        const NFSFS_RENAMED = 0x1000;
        const FSNOTIFIY_PARENT_WATCHED = 0x2000;
        const DENTRY_KILLED = 0x4000;
        const MOUNTED = 0x8000;
        const NEED_AUTOMOUNT = 0x10000;
        const MANAGE_TRANSIT = 0x20000;
    }
}

pub struct DEntry
{
    pub d_flags : DEntryFlags,
    pub d_seq : RWLock,
    pub d_sb : *mut LogicalPart,
    d_parent : *mut DEntry,
    pub d_inode : *mut Inode,
    d_children : BTreeMap<String, *mut DEntry>,
    pub d_ref : AtomicI64,
    pub d_op : *mut DEntryOperations
}


impl DEntry
{
    pub fn dget(&mut self) -> *mut Self
    {
        self.d_ref.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        self
    }

    pub const fn is_dir(&self) -> bool
    {
        unsafe
        {
            (*self.d_inode).is_dir()
        }
    }

    pub const fn is_symlink(&self) -> bool
    {
        unsafe {            
            (*self.d_inode).is_symlink()
        }
    }

    pub fn dput(&mut self)
    {
        unsafe
        {
            let prev = self.d_ref.fetch_sub(1, core::sync::atomic::Ordering::AcqRel);
            if prev == 1
            {
                ptr::drop_in_place(self);
                alloc::alloc::dealloc(self as *mut Self as *mut u8, Layout::new::<Self>());
            }
        }
    }

    pub fn empty(parent : *mut DEntry) -> *mut DEntry
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()) as *mut Self;
            (*ptr) = Self { d_seq: RWLock::new(), d_parent: parent, d_inode: null_mut(), d_children: BTreeMap::new(), d_ref: AtomicI64::new(1), d_op: null_mut(), d_sb: null_mut(), d_flags: DEntryFlags::empty() };
            ptr
        }
    }

    pub fn look_up(&mut self, name : &String) -> *mut DEntry
    {
        unsafe
        {
            self.d_seq.rdlock();
            let result = match self.d_children.get(name) {
                Some(child) => 
                {
                    *child
                },
                None => 
                if self.d_children.is_empty() && (*self.d_inode).is_dir()
                {
                    if self.d_children.is_empty()
                    {
                        (*self.d_inode).load_entrys(addr_of_mut!(*self));
                    }
                    match self.d_children.get(name)
                    {
                        Some(child) => *child,
                        None => null_mut()
                    }
                }
                else
                {
                    null_mut()
                }
            };
            self.d_seq.rdunlock();
            result
        }
    }

    pub fn new_child(&mut self, name : &String) -> *mut Self
    {
        self.d_seq.wrlock();
        let child = Self::empty(self as *mut Self);
        self.d_children.insert(name.to_string(), child);
        self.d_seq.wrunlock();
        child
    }
}
