use core::{sync::atomic::AtomicI64, ptr::null_mut, cmp::Ordering, alloc::{GlobalAlloc, Layout}, ffi::c_void};
use alloc::collections::BTreeSet;
use crate::{kernel::{list::ListHead, process, Off}, mm::memory::{MMAP_START, USER_STACK_BOTTOM}, fs::{namei::Fd, file::{File, FS}}};

use super::{page::Pageflags, memory::MEMORY_POOL};

pub struct MMStruct
{
    pub mmap : *mut VMAreaStruct,
    pub mm_rb : BTreeSet<VMAPtrCmp>,
    pub stack : *mut VMAreaStruct,
    pub mmap_cache : *mut VMAreaStruct,
    pub pcb_ptr : *mut process::ProcessControlBlock
}

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct PageFaultErrorCode : u64
    {
        const PRESENT = 0x1; // When set, the page fault was caused by a page-protection violation. When not set, it was caused by a non-present page.
        const WRITE = 0x2; // When set, the page fault was caused by a write access. When not set, it was caused by a read access.
        const USER = 0x4; // When set, the page fault was caused while CPL = 3. This does not necessarily mean that the page fault was a privilege violation.
        const RESERVED_WRITE = 0x8; // When set, the page fault was caused while CPL = 3. This does not necessarily mean that the page fault was a privilege violation.
        const INSTRUCTION_FETCH = 0x10; // When set, the page fault was caused by an instruction fetch. This only applies when the No-Execute bit is supported and enabled.
        const PROTECTION_KEY = 0x20; // When set, the page fault was caused by a protection-key violation. The PKRU register (for user-mode accesses) or PKRS MSR (for supervisor-mode accesses) specifies the protection key rights.
        const SHADOW_STACK = 0x40; // When set, the page fault was caused by a shadow stack access.
        const SOFTWARE_GUARD_EXTENSIONS = 0x4000; //When set, the fault was due to an SGX violation. The fault is unrelated to ordinary paging.
    }
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
        const PROT_KERNEL = 0x40;

        const MAP_SHARED = 0x01;
        const MAP_PRIVATE = 0x02;
        const MAP_LOCKED = 0x2000;		/* pages are locked */
        const MAP_EXECUTABLE = 0x1000;
        const MAP_ANONYMOUS	= 0x20;
        const MAP_POPULATE = 0x008000;	/* populate (prefault) pagetables */
        const MAP_NONBLOCK = 0x010000;	/* do not block on IO */
        const MAP_STACK	= 0x020000;	/* give out an address that is best suited for process/thread stacks */
        const MAP_HUGETLB = 0x040000;	/* create a huge page mapping */
        const MAP_SYNC = 0x080000; /* perform synchronous page faults for the mapping */
        const MAP_FIXED_NOREPLACE = 0x100000;	/* MAP_FIXED which doesn't unmap underlying mapping */
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
    file : *mut File,
    offset : Off,
    vm_page_prot : u64
}

impl MMStruct {
    pub fn release_all(&mut self)
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
            if (*vm_ptr).get_start() > start as u64
            {
                return self.create_new_mem_area(start as u64, start as u64 + length as u64);
            }
            while !vm_ptr.is_null() {
                if (*vm_ptr).get_start() > start as u64 && (*last_ptr).get_end() as usize + 1  + length < max as usize
                {
                    while !last_ptr.is_null() && (((*vm_ptr).get_start() - (*last_ptr).get_end()) as usize) < length && (*vm_ptr).get_start() as usize + length < max as usize
                    {
                        last_ptr = vm_ptr;
                        vm_ptr = (*vm_ptr).get_next();
                        continue;
                    }
                    if !last_ptr.is_null() && ((((*vm_ptr).get_start() - (*last_ptr).get_end()) as usize) > length || (*last_ptr).get_end() as usize + 1 + length > max as usize)
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
            if (*last_ptr).get_end() < max as u64
            {
                return self.create_new_mem_area((*last_ptr).get_end() + 1, (*last_ptr).get_end() + length as u64);
            }
            null_mut()
        }
    }

    pub fn new(pcb_ptr : *mut process::ProcessControlBlock) -> MMStruct
    {
        MMStruct { mmap: null_mut(), mm_rb: BTreeSet::new(), mmap_cache: null_mut(), pcb_ptr, stack: null_mut() }
    }

    pub fn dispose(mm_ptr : *mut MMStruct)
    {
        todo!()
    }

    pub fn contain(&mut self, addr : u64) -> *mut VMAreaStruct
    {
        unsafe
        {
            let mut vma_ptr = self.mmap;
            while !vma_ptr.is_null() {
                if (*vma_ptr).vm_start < addr
                {
                    if (*vma_ptr).vm_end > addr
                    {
                        return vma_ptr;
                    }
                    else {
                        vma_ptr = (*vma_ptr).get_next();
                    }
                }
                else {
                    break;
                }
            }
            null_mut()
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
            let mut last_ptr = null_mut();
            if vma_ptr.is_null()
            {
                self.mmap = new_vma;
                return new_vma;
            }
            while !vma_ptr.is_null() {
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
                        if vma_ptr.is_null()
                        {
                            self.mmap = new_vma;
                            (*new_vma).set_next(vma_ptr);
                            (*vma_ptr).set_prev(new_vma);
                        }
                        else
                        {
                            if (*vma_ptr).vm_end + 1 == (*new_vma).vm_start && (*vma_ptr).get_file() == (*new_vma).get_file() && (*vma_ptr).get_flags().difference((*new_vma).get_flags()).is_empty() && (*vma_ptr).get_prot() == (*new_vma).get_prot() && (*vma_ptr).get_offset() + ((*vma_ptr).get_end() - (*vma_ptr).get_start() + 1) as Off == (*new_vma).get_offset()
                            {
                                // merge two vma
                                (*vma_ptr).vm_end = (*new_vma).vm_end;
                                Self::free_vma(new_vma);
                                new_vma = vma_ptr;
                            }
                            else {
                                (*(*vma_ptr).get_next()).set_prev(new_vma);
                                (*new_vma).set_next((*vma_ptr).get_next());
                                (*vma_ptr).set_next(new_vma);
                                (*new_vma).set_prev(vma_ptr);
                            }
                        }
                        return new_vma;
                    }
                    Some(Ordering::Less) =>
                    {
                        last_ptr = vma_ptr;
                        vma_ptr = (*vma_ptr).get_next();
                    }
                    None => panic!("vitrual memory arna overlapped!"),
                }
            }
            if (*last_ptr).vm_end + 1 == (*new_vma).vm_start && (*last_ptr).get_file() == (*new_vma).get_file() && (*last_ptr).get_flags().difference((*new_vma).get_flags()).is_empty() && (*last_ptr).get_prot() == (*new_vma).get_prot() && (*last_ptr).get_offset() + ((*last_ptr).get_end() - (*last_ptr).get_start() + 1) as Off == (*new_vma).get_offset()
            {
                if (*last_ptr).get_prev() == last_ptr
                {
                    (*last_ptr).set_next((*new_vma).get_next());
                    (*(*new_vma).get_next()).set_prev(last_ptr);
                }
                Self::free_vma(new_vma);
                new_vma = last_ptr;
            }
            else {
                (*last_ptr).set_next(new_vma);
                (*new_vma).set_prev(last_ptr);
            }
            new_vma
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

    pub fn get_prot(&self) -> u64
    {
        self.vm_page_prot
    }

    pub fn set_prot(&mut self, prot : MmapType)
    {
        self.vm_page_prot = Self::get_vm_page_prot(prot);
    }

    fn get_vm_page_prot(prot : MmapType) -> u64
    {
        Self::arch_get_vm_page_prot(prot)
    }

    fn arch_get_vm_page_prot(prot : MmapType) -> u64
    {
        let mut result = 0x1;
        if prot.contains(MmapType::PROT_WRITE)
        {
            result |= 0x2;
        }
        if !prot.contains(MmapType::PROT_KERNEL)
        {
            result |= 0x4;
        }
        result
    }

    pub fn set_file(&mut self, file_t : *mut File)
    {
        self.file = file_t;            
    }

    pub fn get_file(&self) -> *mut File
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
        VMAreaStruct { vm_start: strat, vm_end: end - 1, list: ListHead::empty(), vm_mm: mm_struct, vm_flags: flags, vm_ref_count: AtomicI64::new(1), file: null_mut(), offset: 0, vm_page_prot: 0 }
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

