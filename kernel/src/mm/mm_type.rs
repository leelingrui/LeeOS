use core::{sync::atomic::AtomicI64, ptr::null_mut, cmp::Ordering, alloc::{GlobalAlloc, Layout}, ffi::c_void};
use alloc::collections::BTreeSet;
use crate::{kernel::{list::ListHead, process, Off}, mm::memory::{MMAP_START, USER_STACK_BOTTOM}, fs::{namei::Fd, file::{FileStruct, FS}}};

use super::{page::Pageflags, memory::MEMORY_POOL};

pub struct MMStruct
{
    pub mmap : *mut VMAreaStruct,
    pub mm_rb : BTreeSet<VMAPtrCmp>,
    pub mmap_cache : *mut VMAreaStruct,
    pub pcb_ptr : *mut process::ProcessControlBlock
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct MmapType : u64
    {
        const PROT_NONE = 0x00000000;

        const PROT_READ = 0x00000001;	/* currently active flags */
        const PROT_WRITE = 0x00000002;
        const PROT_EXEC = 0x00000004;
        const PROT_SHARED = 0x00000008;
        
        /* mprotect() hardcodes VM_MAYREAD >> 4 == VM_READ, and so for r/w/x bits. */
        const VM_MAYREAD = 0x00000010;	/* limits for mprotect() etc */
        const VM_MAYWRITE = 0x00000020;
        const VM_MAYEXEC = 0x00000040;
        const VM_MAYSHARE = 0x00000080;
        
        const VM_GROWSDOWN = 0x00000100;	/* general info on the segment */
        const VM_UFFD_MISSING = 0x00000200;	/* missing pages tracking */
        const VM_MAYOVERLAY = 0x00000200;	/* nommu: R/O MAP_PRIVATE mapping that might overlay a file mapping */
        const VM_PFNMAP	= 0x00000400;	/* Page-ranges managed without "struct page", just pure PFN */
        const VM_UFFD_WP = 0x00001000;	/* wrprotect pages tracking */
        
        const VM_LOCKED = 0x00002000;
        const VM_IO = 0x00004000;	/* Memory mapped I/O or similar */
        
                            /* Used by sys_madvise() */
        const VM_SEQ_READ = 0x00008000;	/* App will access data sequentially */
        const VM_RAND_READ = 0x00010000;	/* App will not benefit from clustered reads */
        
        const VM_DONTCOPY = 0x00020000;      /* Do not copy this vma on fork */
        const VM_DONTEXPAND = 0x00040000;	/* Cannot expand with mremap() */
        const VM_LOCKONFAULT = 0x00080000;	/* Lock the pages covered when they are faulted in */
        const VM_ACCOUNT = 0x00100000;	/* Is a VM accounted object */
        const VM_NORESERVE = 0x00200000;	/* should the VM suppress accounting */
        const VM_HUGETLB = 0x00400000;	/* Huge TLB Page VM */
        const VM_SYNC = 0x00800000;	/* Synchronous page faults */
        const VM_ARCH_1 = 0x01000000;	/* Architecture-specific flag */
        const VM_WIPEONFORK = 0x02000000;	/* Wipe VMA contents in child. */
        const VM_DONTDUMP = 0x04000000;	/* Do not include in the core dump */
        
        const VM_MIXEDMAP = 0x10000000;	/* Can contain "struct page" and pure PFN pages */
        const VM_HUGEPAGE = 0x20000000;	/* MADV_HUGEPAGE marked this vma */
        const VM_NOHUGEPAGE = 0x40000000;	/* MADV_NOHUGEPAGE marked this vma */
        const VM_MERGEABLE = 0x80000000;	/* KSM may merge identical pages */
    }
}

#[derive(Eq)]
pub struct VMAPtrCmp
{
    ptr : *mut VMAreaStruct
}

impl PartialEq for VMAPtrCmp {
    fn eq(&self, other: &Self) -> bool {
        unsafe { *self.ptr == *other.ptr }
    }
}

impl VMAPtrCmp {
    pub fn new(vma_ptr : *mut VMAreaStruct) -> VMAPtrCmp
    {
        VMAPtrCmp { ptr: vma_ptr }
    }
}

impl Ord for VMAPtrCmp {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let result = self.partial_cmp(other);
        match result
        {
            Some(x) => return x,
            None => panic!("vitrual memory arna overlapped!")
        }

    }

    fn max(self, other: Self) -> Self
    where
        Self: Sized,
    {
        core::cmp::max_by(self, other, Ord::cmp)
    }

    fn min(self, other: Self) -> Self
    where
        Self: Sized,
    {
        core::cmp::min_by(self, other, Ord::cmp)
    }

    fn clamp(self, min: Self, max: Self) -> Self
    where
        Self: Sized,
        Self: PartialOrd,
    {
        assert!(min <= max);
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}

impl PartialOrd for VMAPtrCmp {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        unsafe { (*self.ptr).partial_cmp(&(*other.ptr)) }
    }

    fn lt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(core::cmp::Ordering::Less))
    }

    fn le(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal))
    }

    fn gt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(core::cmp::Ordering::Greater))
    }

    fn ge(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(core::cmp::Ordering::Greater | core::cmp::Ordering::Equal))
    }
}

pub struct VMAreaStruct
{
    vm_start : u64,
    vm_end : u64,
    list : ListHead,
    vm_mm : *mut MMStruct,
    vm_flags : MmapType,
    vm_ref_count : AtomicI64,
    file : *mut FileStruct,
    offset : Off,
    vm_page_prot : MmapType
}

impl MMStruct {
    pub fn release_all(&self)
    {
        unsafe
        {
            self.mmap_cache = null_mut();
            self.mm_rb.clear();
            let mut vma_ptr = self.mmap;
            while !vma_ptr.is_null() {

                let prev_vma = vma_ptr;
                vma_ptr = (*vma_ptr).get_next();
                Self::free_vma(prev_vma);
            }
        }
    }

    pub fn scan_empty_space(&mut self, mut start : *const c_void, length : usize, mut max : *const c_void) -> *mut VMAreaStruct
    {
        assert!((start as u64 & 0xfff) == 0);
        assert!((length & 0xfff) == 0);
        unsafe
        {
            if start.is_null()
            {
                start = MMAP_START;
            }
            if max.is_null()
            {
                max = USER_STACK_BOTTOM;
            }
            let mut last_ptr: *mut VMAreaStruct = null_mut();
            let mut vm_ptr = self.mmap;
            while !vm_ptr.is_null() {
                if (*vm_ptr).get_end() < start as u64 && (*vm_ptr).get_end() as usize + 1  + length < max as usize
                {
                    while !vm_ptr.is_null() && (((*vm_ptr).get_start() - (*last_ptr).get_end()) as usize) < length && (*vm_ptr).get_start() as usize + length < max as usize
                    {
                        last_ptr = vm_ptr;
                        vm_ptr = (*vm_ptr).get_next();
                    }
                    if vm_ptr.is_null() || (((*vm_ptr).get_start() - (*last_ptr).get_end()) as usize) > length || (*last_ptr).get_end() as usize + 1 + length > max as usize
                    {
                        if (*last_ptr).get_end() as usize + length > max as usize
                        {
                            return self.create_new_mem_area((*last_ptr).get_end() + 1, (*last_ptr).get_end() + length as u64);
                        }
                        return null_mut();
                    }
                    return self.create_new_mem_area((*last_ptr).get_end() + 1, (*last_ptr).get_end() + length as u64)
                }
                else {
                    last_ptr = vm_ptr;
                    vm_ptr = (*vm_ptr).get_next();
                }
            }
            null_mut()
        }
    }

    pub fn new(pcb_ptr : *mut process::ProcessControlBlock) -> MMStruct
    {
        unsafe
        {
            MMStruct { mmap: null_mut(), mm_rb: BTreeSet::new(), mmap_cache: null_mut(), pcb_ptr }
        }
    }

    pub fn dispose(mm_ptr : *mut MMStruct)
    {
        todo!()
    }

    pub fn contain(&mut self, addr : u64) -> bool
    {
        unsafe
        {
            let mut vma_ptr = self.mmap;
            loop {
                if (*vma_ptr).vm_start < addr
                {
                    if (*vma_ptr).vm_end > addr
                    {
                        return true;
                    }
                    else {
                        vma_ptr = (*vma_ptr).get_next();
                    }
                }
                else {
                    break;
                }
            }
            false
        }

    }

    fn free_vma(vma_ptr : *const VMAreaStruct)
    {
        unsafe
        {
            MEMORY_POOL.dealloc(vma_ptr as *mut u8, Layout::new::<VMAreaStruct>())
        }
    }

    pub fn create_new_mem_area(&mut self, start : u64, end : u64) -> *mut VMAreaStruct
    {
        unsafe
        {
            let vma_ptr = MEMORY_POOL.alloc(Layout::new::<VMAreaStruct>()) as *mut VMAreaStruct;
            (*vma_ptr) = VMAreaStruct::new(start, end, self as *mut MMStruct, MmapType::empty());
            self.insert_vma(vma_ptr);
            vma_ptr
        }
    }

    fn insert_vma(&mut self, mut new_vma : *mut VMAreaStruct) -> *mut VMAreaStruct
    {
        unsafe {
            let mut vma_ptr = self.mmap;
            if vma_ptr.is_null()
            {
                self.mmap = new_vma;
                return new_vma;
            }
            loop {
                let result = (*vma_ptr).partial_cmp(&*new_vma);
                match result {
                    Some(Ordering::Equal) => 
                    {
                        panic!("vitrual memory arna overlapped!")
                    },
                    Some(Ordering::Greater) =>
                    {
                        if (*vma_ptr).vm_start == (*new_vma).vm_end + 1 && (*new_vma).vm_flags.difference((*vma_ptr).vm_flags).is_empty()
                        {
                            (*vma_ptr).vm_start = (*new_vma).vm_start;
                            Self::free_vma(new_vma);
                            new_vma = vma_ptr;
                        }
                        vma_ptr = (*vma_ptr).get_prev();
                        if (*vma_ptr).vm_end + 1 == (*new_vma).vm_start
                        {
                            (*vma_ptr).vm_end = (*vma_ptr).vm_end;
                            if (*vma_ptr).get_prev() == vma_ptr
                            {
                                (*vma_ptr).set_next((*new_vma).get_next());
                                (*(*new_vma).get_next()).set_prev(vma_ptr);
                            }
                            Self::free_vma(new_vma);
                            new_vma = vma_ptr;
                        }
                        else {
                            (*(*vma_ptr).get_next()).set_prev(new_vma);
                            (*new_vma).set_next((*vma_ptr).get_next());
                            (*vma_ptr).set_next(new_vma);
                            (*new_vma).set_prev(vma_ptr);
                        }
                        return new_vma;
                    }
                    Some(Ordering::Less) =>
                    {
                        vma_ptr = (*vma_ptr).get_next();
                    }
                    None => panic!("vitrual memory arna overlapped!"),
                }
            }
        }
    }
}

impl PartialEq for VMAreaStruct
{
    fn eq(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(core::cmp::Ordering::Equal))
    }
}

impl PartialOrd for VMAreaStruct
{
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        if self.vm_start < other.vm_start
        {
            if self.vm_end < other.vm_start
            {
                return Some(core::cmp::Ordering::Less);
            }
            else {
                return  Option::None;
            }
        }
        else {
            if self.vm_end <= other.vm_end
            {
                return Some(core::cmp::Ordering::Equal);
            }
            if self.vm_start > other.vm_end
            {
                return Some(core::cmp::Ordering::Greater);
            }
        }
        Option::None
    }

    fn lt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(core::cmp::Ordering::Less))
    }

    fn le(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(core::cmp::Ordering::Less | core::cmp::Ordering::Equal))
    }

    fn gt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(core::cmp::Ordering::Greater))
    }

    fn ge(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(core::cmp::Ordering::Greater | core::cmp::Ordering::Equal))
    }
}

impl VMAreaStruct {
    pub fn set_offset(&mut self, offset : Off)
    {
        self.offset = offset;
    }

    pub fn get_offset(&self) -> Off
    {
        self.offset
    }

    pub fn get_flags(&self) -> MmapType
    {
        self.vm_flags
    }

    pub fn set_flags(&mut self, flags : MmapType)
    {
        self.vm_flags = flags;
    }

    pub fn get_prot(&self) -> MmapType
    {
        self.vm_page_prot
    }

    pub fn set_prot(&mut self, prot : MmapType)
    {
        self.vm_page_prot = prot;
    }

    pub fn set_file(&mut self, file_t : *mut FileStruct)
    {
        unsafe {
            FS.release_file(self.file);
            (*file_t).count.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            self.file = file_t;            
        }
    }

    pub fn get_file(&self) -> *mut FileStruct
    {
        self.file
    }

    pub fn get_start(&self) -> u64
    {
        self.vm_start
    }

    pub fn get_end(&self) -> u64
    {
        self.vm_end
    }

    pub fn new(strat : u64, end : u64, mm_struct : *mut MMStruct, flags : MmapType) -> VMAreaStruct
    {
        VMAreaStruct { vm_start: strat, vm_end: end - 1, list: ListHead::empty(), vm_mm: mm_struct, vm_flags: flags, vm_ref_count: AtomicI64::new(1), file: null_mut(), offset: 0, vm_page_prot: MmapType::empty() }
    }

    pub fn get_next(&self) -> *mut VMAreaStruct
    {
        self.list.next as *mut VMAreaStruct
    }

    pub fn get_prev(&self) -> *mut VMAreaStruct
    {
        self.list.prev as *mut VMAreaStruct
    }

    pub fn set_next(&mut self, next : *mut VMAreaStruct)
    {
        self.list.next = next as *mut ListHead;
    }

    pub fn set_prev(&mut self, prev : *mut VMAreaStruct)
    {
        self.list.prev = prev as *mut ListHead;
    }
}

