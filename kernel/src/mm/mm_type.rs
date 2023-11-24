use core::{sync::atomic::AtomicI64, ptr::null_mut, cmp::Ordering, alloc::{GlobalAlloc, Layout}};
use alloc::collections::BTreeSet;
use crate::kernel::{list::ListHead, process};

use super::{page::Pageflags, memory::MEMORY_POOL};

pub struct MMStruct
{
    mmap : *mut VMAreaStruct,
    mm_rb : BTreeSet<VMAPtrCmp>,
    mmap_cache : *mut VMAreaStruct,
    pcb_ptr : *mut process::ProcessControlBlock
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
    vm_flags : Pageflags,
    vm_ref_count : AtomicI64,
}

impl MMStruct {
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

    pub fn create_new_mem_area(&mut self, start : u64, end : u64)
    {
        unsafe
        {
            let vma_ptr = MEMORY_POOL.alloc(Layout::new::<VMAreaStruct>()) as *mut VMAreaStruct;
            (*vma_ptr) = VMAreaStruct::new(start, end, self as *mut MMStruct, Pageflags::PgActive);
            self.insert_vma(vma_ptr);
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
                        if (*vma_ptr).vm_start == (*new_vma).vm_end + 1 && (*new_vma).vm_flags == (*vma_ptr).vm_flags
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
    pub fn new(strat : u64, end : u64, mm_struct : *mut MMStruct, flags : Pageflags) -> VMAreaStruct
    {
        VMAreaStruct { vm_start: strat, vm_end: end, list: ListHead::empty(), vm_mm: mm_struct, vm_flags: flags, vm_ref_count: AtomicI64::new(1) }
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

