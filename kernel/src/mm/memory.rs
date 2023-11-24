use alloc::alloc::Layout;
use alloc::boxed::Box;
use bitflags::Flag;
use core::alloc::GlobalAlloc;
use core::fmt::Display;
use core::intrinsics::{log2f64, size_of};
use core::ops::Range;
use core::ptr::{null_mut, null};
use core::{ffi::c_void, arch::asm, fmt};

use bitfield::bitfield;
use buddy_system_allocator::{LockedFrameAllocator, LockedHeap};

use crate::kernel::cpu::{get_cr2_reg, get_cpu_number};
use crate::kernel::interrupt::set_interrupt_handler;
use crate::kernel::{interrupt, sched};
use crate::kernel::sched::get_current_running_process;
use crate::{printk, logk, bochs_break};


use crate::kernel::cpu;
use super::page::{self, Pageflags, GFP};
use super::slub;
use crate::kernel::process::PtRegs;
use crate::kernel::{relocation, bitmap, string::memset, semaphore};
const ARDS_BUFFER : *const c_void = 0x7c00 as *const c_void;
static mut KERNEL_PAGE_DIR : *const c_void = 0x0 as *const c_void;
pub static mut MEMORY_DESCRIPTOR : MemoryDescriptor = MemoryDescriptor{ size : 0, all_pages : 0, start : core::ptr::null() };
#[global_allocator]
pub static mut MEMORY_POOL : MemoryPool = MemoryPool::new();
pub const PAGE_SHIFT : usize = 12;
pub const PAGE_SIZE : usize = 1 << 12;
const MAX_ORDER : usize = 11;
const KERNEL_START : usize = 0xffff800000100000;
const VIRTADDR_START : usize = 0xffff800000000000;
const PHYADDR_START : *mut c_void = 0x100000 as *mut c_void;
pub const LINEAR_MAP_ARREA_START : *mut c_void = 0xffff880000000000 as *mut c_void;
pub const LINEAR_MAP_ARREA_END : *mut c_void = 0xffffc80000000000 as *mut c_void;
pub const USER_STACK_START : *mut c_void = 0x00007ffffffff000 as *mut c_void;
pub fn handle_alloc_error(layout : Layout) -> !
{
    panic!("heap alloction error, layout = {:?}", layout);
}
unsafe impl GlobalAlloc for MemoryPool {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if layout.size() <= 2048
        {
            let kmem_cache = slub::kmalloc_slab(layout.size(), GFP::empty());
            (*kmem_cache).alloc() as *mut u8
        }
        else {
            let need_pages = (layout.size() / PAGE_SIZE) + ((layout.size() % PAGE_SIZE != 0) as usize);
            MEMORY_POOL.alloc_frames(need_pages) as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        // SAFETY: the safety contract for `alloc` must be upheld by the caller.
        let ptr = unsafe { self.alloc(layout) };
        if !ptr.is_null() {
            // SAFETY: as allocation succeeded, the region from `ptr`
            // of size `size` is guaranteed to be valid for writes.
            unsafe { core::ptr::write_bytes(ptr, 0, size) };
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: the caller must ensure that the `new_size` does not overflow.
        // `layout.align()` comes from a `Layout` and is thus guaranteed to be valid.
        let new_layout = unsafe { Layout::from_size_align_unchecked(new_size, layout.align()) };
        // SAFETY: the caller must ensure that `new_layout` is greater than zero.
        let new_ptr = unsafe { self.alloc(new_layout) };
        if !new_ptr.is_null() {
            // SAFETY: the previously allocated block cannot overlap the newly allocated block.
            // The safety contract for `dealloc` must be upheld by the caller.
            unsafe {
                core::ptr::copy_nonoverlapping(ptr, new_ptr, core::cmp::min(layout.size(), new_size));
                self.dealloc(ptr, layout);
            }
        }
        new_ptr
    }
}
pub struct MemoryPool
{
    pub mem_map : *mut page::Page,
    lowest_idx : usize,
    free_pages : usize,
    frame_allocator : *mut LockedFrameAllocator
}

// struct BuddySystem
// {
//     bucket : [MemorySpan; MAX_ORDER],
//     lock : semaphore::SpinLock,
//     current_vmemory : *mut c_void
// }

// impl BuddySystem {
//     fn new(start_vaddr : *mut c_void) ->BuddySystem
//     {
//         BuddySystem { bucket:[MemorySpan::new(); MAX_ORDER], lock: semaphore::SpinLock::new(1), current_vmemory: start_vaddr }
//     }

//     fn alloc(&mut self, layout : Layout) -> *mut c_void
//     {
//         let mut ptr = self.get_page_from_bucket(layout);
//         if ptr.is_null()
//         {
//             ptr = self.get_vpage(layout);
//         }
//         ptr
//     }

//     fn get_bucket(size : usize) -> usize
//     {
//         let mut result = unsafe { log2f64(size as f64 / PAGE_SIZE as f64) };
//         if (result % 1.0).ne(&0.0)
//         {
//             result += 1.0;
//         }
//         result as usize
//     }
    
//     fn check_align(layout : Layout, start_vaddr : *mut c_void) -> bool
//     {
//         start_vaddr as usize % layout.align() == 0
//     }

//     fn get_page_from_bucket(&mut self, layout : Layout) -> *mut c_void
//     {   
//         let best_bucket = Self::get_bucket(layout.size());
//         unsafe { 
//             let mut current_span = &mut self.bucket[best_bucket] as *mut MemorySpan;
            
//             loop {
//                 if Self::check_align(layout, current_span as *mut c_void)
//                 {
//                     (*(*current_span).prev).next = (*current_span).next;
//                     (*(*current_span).next).prev = (*current_span).prev;
//                     break;
//                 }
//                 else {
//                     current_span = (*current_span).next;
//                 }
//             }
//             if current_span.is_null()
//             {
//                 current_span = self.split_bucket(best_bucket, layout);
//             }
//             current_span as *mut c_void
//         }
//     }

//     fn insert(&mut self, memory_span : *mut MemorySpan)
//     {
//         unsafe
//         {
//             (*memory_span).next = (self.bucket[(*memory_span).size_level]).next;
//             (*(self.bucket[(*memory_span).size_level]).next).prev = memory_span;
//             (*memory_span).prev = &mut self.bucket[(*memory_span).size_level] as *mut MemorySpan;
//             self.bucket[(*memory_span).size_level].next = memory_span;
//         }
//     }

//     fn split_bucket(&mut self, current_order : usize, layout : Layout) -> *mut MemorySpan
//     {
//         let current_span = self.bucket[current_order + 1].next;
//         let result;
//         unsafe
//         {
//             while !current_span.is_null()
//             {
//                 if Self::check_align(layout, current_order as *mut c_void)
//                 {
//                     (*(*current_span).prev).next = (*current_span).next;
//                     (*(*current_span).next).prev = (*current_span).prev;
//                     result = current_span;
//                     break;
//                 }
//                 else if Self::check_align(layout, (current_order as *mut c_void).offset((current_order * PAGE_SIZE) as isize)){
//                     (*(*current_span).prev).next = (*current_span).next;
//                     (*(*current_span).next).prev = (*current_span).prev;
//                     result = current_span.offset((current_order * PAGE_SIZE) as isize);
//                     break;
//                 }
//             }
//         }
//         result
//     }
    
//     fn get_vpage(&mut self, layout : Layout) -> *mut c_void
//     {
//         self.lock.acquire(1);
//         let result = self.current_vmemory;
//         unsafe { self.current_vmemory = self.current_vmemory.offset((PAGE_SIZE * layout.size().div_ceil(PAGE_SIZE)) as isize) };
//         self.lock.release(1);
//         result
//     }
// }
#[inline]
pub fn phys2page(phys : *const c_void) -> u64
{
    (phys.wrapping_sub(PHYADDR_START as usize) as u64) >> PAGE_SHIFT
}

#[inline]
pub fn phys2virt(paddr : *const c_void) -> *mut c_void
{
    page2virt(phys2page(paddr))
}
#[inline]
pub fn virt2page(addr : *const c_void) -> u64
{
    (addr.wrapping_sub(LINEAR_MAP_ARREA_START as usize + PHYADDR_START as usize) as u64) >> PAGE_SHIFT as u64
}

#[inline]
pub fn page2phys(page : u64) -> *mut c_void
{
    (PHYADDR_START as u64 + (page << PAGE_SHIFT)) as *mut c_void
}

#[inline]
pub fn page2virt(page : u64) -> *mut c_void
{
    unsafe { ((LINEAR_MAP_ARREA_START as u64 + (page << PAGE_SHIFT)) as *mut c_void).offset(PHYADDR_START as isize) }
}

#[inline]
pub fn virt2phys(virt : *const c_void) -> *mut c_void
{
    page2phys(virt2page(virt))
}

#[inline]
pub fn get_free_pointer(kmem_struck : &slub::KmemCache, object : *const c_void) -> *mut c_void
{
    unsafe { *(object.offset(kmem_struck.offset as isize) as *mut *mut c_void) }
}

#[inline]
pub fn set_free_pointer(kmem_struck : &slub::KmemCache, object : *const c_void, fp : *const c_void)
{
    unsafe {
        (*(object.offset(kmem_struck.offset.try_into().unwrap()) as *mut *const c_void)) = fp.offset(kmem_struck.offset.try_into().unwrap());
    }
}

impl MemoryPool {
    pub fn kmalloc_bootstrap(&mut self)
    {
        unsafe
        {
            let all_size = size_of::<slub::KmemCache>() * slub::KMALLOC_CACHES_NUM;
            let need_pages = all_size.div_ceil(PAGE_SIZE);
            let first_page_vaddr = self.alloc_frame_temporary();
            slub::SLAB_CACHES = first_page_vaddr as *mut slub::KmemCache;
            let mut var = 1;
            // allocate init pages
            while var < need_pages
            {
                self.alloc_frame_temporary();
                var += 1; 
            }
            let page_id = virt2page(first_page_vaddr) as usize;
            var = 0;
            while var < need_pages {
                (*self.mem_map.offset((page_id + var) as isize))._refcount.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                var += 1;
            }
            Self::create_kmem_caches(first_page_vaddr);
            Self::link_kmem_caches();
        }
    }

    fn link_kmem_caches()
    {
        unsafe
        {
            let mut var = 0;
            while  var < slub::KMALLOC_CACHES_NUM - 1 {
                (*slub::SLAB_CACHES.offset(var as isize)).set_next(slub::SLAB_CACHES.offset((var + 1) as isize));
                (*slub::SLAB_CACHES.offset((var + 1) as isize)).set_prev(slub::SLAB_CACHES.offset(var as isize));
                var += 1
            }
            (*slub::SLAB_CACHES.offset(0)).set_next(null_mut());
            (*slub::SLAB_CACHES.offset((slub::KMALLOC_CACHES_NUM - 1) as isize)).set_next(null_mut());
            // var = 0;
            // while var < slub::KMALLOC_CACHES_NUM {
            //     slub
            //     var += 1;
            // }
        }
    }

    fn create_kmem_caches(vaddr : *const c_void)
    {
        unsafe {
            let mut kmem_cache_ptr = vaddr as *mut slub::KmemCache;
            let mut var = 0;
            while var < slub::KMALLOC_CACHES_NUM {
                (*kmem_cache_ptr) = slub::KmemCache::create_cache(slub::KMALLOC_INFO[var].name as *const str, slub::KMALLOC_INFO[var].size, slub::KMALLOC_INFO[var].size, null_mut(), Pageflags::PgSlab);
                slub::KMALLOC_CACHES[var] = kmem_cache_ptr;
                for node in &mut (*kmem_cache_ptr).node
                {
                    node.kmem_cache_node_bootstrap(&(*kmem_cache_ptr));
                }
                var += 1;
                kmem_cache_ptr = kmem_cache_ptr.offset(1);
            }
        }
    }

    pub fn alloc_frames(&mut self, page_num : usize) -> *mut c_void
    {
        unsafe
        {
            let page_frame = (*self.frame_allocator).lock().alloc(page_num);
            match &page_frame
            {
                None => return null_mut(),
                Some(start_frame) =>
                {
                    let mut var = 0;
                    while var < page_num {
                        (*self.mem_map.offset((start_frame + var) as isize))._refcount.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                        var += 1;
                    }
                    page2virt(*start_frame as u64)
                }
            }
        }
    }

    const fn new() -> MemoryPool
    {
        let memory_pool = MemoryPool{ mem_map : null_mut(), lowest_idx : 0, free_pages : 0, frame_allocator: null_mut() }; // kernel_vmem_pool: BuddySystem { bucket: [MemorySpan::new(); MAX_ORDER], lock: semaphore::SpinLock::new(1), current_vmemory: null_mut() } 
        memory_pool
    }
    fn init(&mut self, memory_descriptor : &mut MemoryDescriptor)
    {
        assert!(size_of::<slub::Slab>() == size_of::<page::Page>(), "all page descriptor must have same length");
        unsafe {
            // self.page_map.reset_bitmap(memory_descriptor.start.offset((get_kernel_size() + KERNEL_START) as isize) as *mut u8, MEMORY_DESCRIPTOR.all_pages);
            self.mem_map = (relocation::KERNEL_SIZE) as *mut page::Page;
            self.free_pages = MEMORY_DESCRIPTOR.all_pages;
            let mut pml4_position = relocation::KERNEL_SIZE + MEMORY_DESCRIPTOR.all_pages * size_of::<page::Page>(); // calculate PDPTE virtual position
            pml4_position = ((pml4_position / PAGE_SIZE) + ((pml4_position % PAGE_SIZE) != 0) as usize) * PAGE_SIZE;
            let used_page = (pml4_position - 0xffff800000100000) / PAGE_SIZE + 1;
            compiler_builtins::mem::memset(self.mem_map as *mut u8, 0, size_of::<page::Page>() * MEMORY_DESCRIPTOR.all_pages);
            self.free_pages -= used_page;
            self.lowest_idx += used_page;
            self.init_pml4(pml4_position as *mut Pml4, pml4_position - KERNEL_START + 0x100000 + PAGE_SIZE, (KERNEL_START - 0x100000) as *mut c_void, 0x0 as *mut c_void);
            Self::init_linear_map_area(pml4_position as *mut Pml4);
            self.init_used_page_counter(used_page + 1);
            printk!("Pml4: {}", (*(pml4_position as *mut Pml4)).entry[272]);
            pml4_position -= 0xffff800000000000; // pdpte physical position
            // (self.frame_allocator).unwrap().lock().insert(core::ops::Range { start:pml4_position + PAGE_SIZE, end: memory_descriptor.all_pages * PAGE_SIZE + 0x100000 });
            //(*self.frame_allocator).lock().add_frame(start, end);
            set_cr3_reg(pml4_position as *const c_void);
            self.kmalloc_bootstrap();
            self.frame_allocator = Box::leak(Box::new(LockedFrameAllocator::new()));
            (*self.frame_allocator).lock().insert(Range{start: self.lowest_idx, end: self.lowest_idx + 32 });
            (*self.frame_allocator).lock().insert(Range{start: self.lowest_idx + 32, end: MEMORY_DESCRIPTOR.all_pages });
        }
    }
    #[no_mangle]
    fn get_page_idx(p_addr : *const c_void) -> u64
    {
        p_addr as u64 >> 12 & 0xfffffffff
    }

    fn init_used_page_counter(&self, pages : usize)
    {
        unsafe
        {
            let mut var = 0;
            while var < pages {
                (*self.mem_map.offset(var as isize))._refcount.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
                var += 1;
            }
        }
    }

    fn init_pml4(&mut self, pml4_ptr : *mut Pml4, mut total_kernel_size : usize, mut dst_vaddr : *mut c_void, mut dst_paddr : *mut c_void)
    {
        unsafe {
            let mut var = get_pml4_offset(dst_vaddr);
            while total_kernel_size >> 39 != 0
            {
                let pdpt_addr = virt2phys(self.alloc_frame_temporary());
                memset(pdpt_addr as *mut u8, 0, PAGE_SIZE);
                (*pml4_ptr).entry[var].set_page_offset(Self::get_page_idx(pdpt_addr));
                (*pml4_ptr).entry[var].set_present(1);
                (*pml4_ptr).entry[var].set_wr(1);
                (*pml4_ptr).entry[var].set_us(1);
                self.init_pdpt(pdpt_addr as *mut Pdpt, &mut total_kernel_size, &mut dst_vaddr, &mut dst_paddr);
                var += 1;
            }
            let pdpt_addr = virt2phys(self.alloc_frame_temporary());
            memset(pdpt_addr as *mut u8, 0, PAGE_SIZE);
            (*pml4_ptr).entry[var].set_page_offset(Self::get_page_idx(pdpt_addr));
            (*pml4_ptr).entry[var].set_present(1);
            (*pml4_ptr).entry[var].set_wr(1);
            (*pml4_ptr).entry[var].set_us(1);
            self.init_pdpt(pdpt_addr as *mut Pdpt, &mut total_kernel_size, &mut dst_vaddr, &mut dst_paddr);
        }
    }

    fn set_pml4(pdpt_ptr : *mut Pdpt, dst_vaddr : *const c_void, dst_paddr : *const c_void, big_page : bool, kernel_space : bool, writable : bool)
    {
        unsafe
        {
            let var = get_pml4_offset(dst_vaddr);
            if (*pdpt_ptr).entry[var].get_present() == 0
            {
                (*pdpt_ptr).entry[var].set_page_offset(Self::get_page_idx(dst_paddr));
                (*pdpt_ptr).entry[var].set_present(1);
                (*pdpt_ptr).entry[var].set_wr(writable as u64);
                (*pdpt_ptr).entry[var].set_us(!kernel_space as u64);
                (*pdpt_ptr).entry[var].set_ps(big_page as u64);
            }
        }
    }

    fn set_pdpt(pdpt_ptr : *mut Pdpt, dst_vaddr : *const c_void, dst_paddr : *const c_void, big_page : bool, kernel_space : bool, writable : bool)
    {
        unsafe
        {
            let var = get_pdpt_offset(dst_vaddr);
            if (*pdpt_ptr).entry[var].get_present() == 0
            {
                (*pdpt_ptr).entry[var].set_page_offset(Self::get_page_idx(dst_paddr));
                (*pdpt_ptr).entry[var].set_present(1);
                (*pdpt_ptr).entry[var].set_wr(writable as u64);
                (*pdpt_ptr).entry[var].set_us(!kernel_space as u64);
                (*pdpt_ptr).entry[var].set_ps(big_page as u64);
            }
        }
    }

    fn set_pdt(pdt_ptr : *mut Pdpt, dst_vaddr : *const c_void, dst_paddr : *const c_void, big_page : bool, kernel_space : bool, writable : bool)
    {
        unsafe
        {
            let var = get_pdt_offset(dst_vaddr);
            if (*pdt_ptr).entry[var].get_present() == 0
            {
                (*pdt_ptr).entry[var].set_page_offset(Self::get_page_idx(dst_paddr));
                (*pdt_ptr).entry[var].set_present(1);
                (*pdt_ptr).entry[var].set_wr(writable as u64);
                (*pdt_ptr).entry[var].set_us(!kernel_space as u64);
                (*pdt_ptr).entry[var].set_ps(big_page as u64);
            }
        }
    }

    fn init_pdpt(&mut self, pdpt_ptr : *mut Pdpt, total_size : &mut usize, dst_vaddr : &mut *mut c_void, dst_paddr : &mut *mut c_void)
    {
        unsafe
        {
            let mut var = get_pdpt_offset(*dst_vaddr);
            while *total_size >> 30 != 0 {
                let pdt_addr = virt2phys(self.alloc_frame_temporary());
                memset(pdt_addr as *mut u8, 0, PAGE_SIZE);
                (*pdpt_ptr).entry[var].set_page_offset(Self::get_page_idx(pdt_addr));
                (*pdpt_ptr).entry[var].set_present(1);
                (*pdpt_ptr).entry[var].set_wr(1);
                (*pdpt_ptr).entry[var].set_us(1);
                self.init_pdt(pdpt_ptr as *mut Pdt, total_size, dst_vaddr, dst_paddr);
                var += 1;
            }
            let pdt_addr = virt2phys(self.alloc_frame_temporary());
            memset(pdt_addr as *mut u8, 0, PAGE_SIZE);
            (*pdpt_ptr).entry[var].set_page_offset(Self::get_page_idx(pdt_addr));
            (*pdpt_ptr).entry[var].set_present(1);
            (*pdpt_ptr).entry[var].set_wr(1);
            (*pdpt_ptr).entry[var].set_us(1);
            self.init_pdt(pdt_addr as *mut Pdt, total_size, dst_vaddr, dst_paddr);
        }
    }

    fn init_linear_map_area(pml4_ptr : *mut Pml4)
    {
        unsafe {
            let mut vaddr = LINEAR_MAP_ARREA_START as u64;
            let mut paddr = null() as *const c_void;
            while (vaddr < LINEAR_MAP_ARREA_END  as u64) && ((paddr as usize) < MEMORY_DESCRIPTOR.size) {
                let var = get_pml4_offset(vaddr as *mut c_void);
                let pdpt_ptr = MEMORY_POOL.alloc_frame_temporary();
                if (cpu::__cpuid(0x80000001).edx & cpu::SUPPORT_1GB_PAGE) != 0
                {
                    Self::set_pdpt(pdpt_ptr as *mut Pdpt, vaddr as *const c_void, paddr, true, true, true);
                    vaddr += 1 << 30;
                    paddr = paddr.offset(1 << 30);
                }
                else
                {
                    let pdt_ptr = MEMORY_POOL.alloc_frame_temporary();
                    compiler_builtins::mem::memset(pdt_ptr as *mut u8, 0, PAGE_SIZE);
                    Self::set_pdpt(pdpt_ptr as *mut Pdpt, vaddr as *const c_void, virt2phys(pdt_ptr) as *const c_void, false, true, true);
                    let mut var = 0;
                    while var < 512 {
                        Self::set_pdt(pdt_ptr as *mut Pdt, vaddr as *const c_void, paddr, true, false, true);
                        vaddr += 1 << 21;
                        paddr = paddr.offset(1 << 21);
                        var += 1;
                    }
                }
                (*pml4_ptr).entry[var].set_page_offset(Self::get_page_idx(virt2phys(pdpt_ptr as *const c_void)));
                (*pml4_ptr).entry[var].set_present(1);
                (*pml4_ptr).entry[var].set_wr(1);
                (*pml4_ptr).entry[var].set_us(1);
                (*pml4_ptr).entry[var].set_ps(0);
                paddr = paddr.offset(1 << 30);
            }

        }

    }

    fn init_pdt(&mut self, pdt_ptr : *mut Pdpt, total_size : &mut usize, dst_vaddr : &mut *mut c_void, dst_paddr : &mut *mut c_void)
    {
        unsafe
        {
            let mut var = get_pdt_offset(*dst_vaddr);
            while *total_size >> 21 != 0 {
                let pt_addr = virt2phys(self.alloc_frame_temporary());
                memset(pt_addr as *mut u8, 0, PAGE_SIZE);
                (*pdt_ptr).entry[var].set_page_offset(Self::get_page_idx(pt_addr));
                (*pdt_ptr).entry[var].set_present(1);
                (*pdt_ptr).entry[var].set_wr(1);
                (*pdt_ptr).entry[var].set_us(1);
                self.init_pt(pt_addr as *mut Pt, total_size, dst_vaddr, dst_paddr);
                var += 1;
            }
            let pt_addr = virt2phys(self.alloc_frame_temporary());
            memset(pt_addr as *mut u8, 0, PAGE_SIZE);
            (*pdt_ptr).entry[var].set_page_offset(Self::get_page_idx(pt_addr));
            (*pdt_ptr).entry[var].set_present(1);
            (*pdt_ptr).entry[var].set_wr(1);
            (*pdt_ptr).entry[var].set_us(1);
            self.init_pt(pt_addr as *mut Pt, total_size, dst_vaddr, dst_paddr);
        }
    }

    fn init_pt(&mut self, pt_ptr : *mut Pt, total_size : &mut usize, dst_vaddr : &mut *mut c_void, dst_paddr : &mut *mut c_void)
    {
        unsafe
        {
            if *total_size == 0
            {
                return;
            }
            let mut var = get_pt_offset(*dst_vaddr);
            loop {
                Self::set_pt(&mut (*pt_ptr).entry[var], *dst_paddr, true, true, true, false, false, false, false, false, false);
                *dst_paddr = dst_paddr.offset(1 << 12);
                var += 1;
                if *total_size <= 1 << 12 || var >= 512
                {
                    if *total_size <= 1 << 12
                    {
                        *total_size = 0;
                    }
                    break;
                }
                *total_size -= 1 << 12;
            }
        }
    }

    fn set_pt(pt_entry : &mut PtEntry, dst_paddr : *const c_void, present : bool, writable : bool, every_one_avaliable : bool, pwt : bool, pcd : bool, accessed : bool, dirty : bool, pat : bool, global : bool)
    {
        pt_entry.set_present(present.try_into().unwrap());
        pt_entry.set_wr(writable.try_into().unwrap());
        pt_entry.set_us(every_one_avaliable.try_into().unwrap());
        pt_entry.set_pwt(pwt.try_into().unwrap());
        pt_entry.set_pcd(pcd.try_into().unwrap());
        pt_entry.set_accessed(accessed.try_into().unwrap());
        pt_entry.set_dirty(dirty.try_into().unwrap());
        pt_entry.set_pat(pat.try_into().unwrap());
        pt_entry.set_global(global.try_into().unwrap());
        pt_entry.set_page_offset(Self::get_page_idx(dst_paddr));
    }
    

    pub fn alloc_frame_temporary(&mut self) -> *mut c_void
    {
        if self.free_pages == 0
        {
            return null_mut();
        }
        unsafe
        {
            loop {
                if (*self.mem_map.offset(self.lowest_idx as isize))._refcount.load(core::sync::atomic::Ordering::Relaxed) != 0
                {
                    self.lowest_idx += 1;
                }
                else {
                    break;
                }
            }
        }
        unsafe {
            (*self.mem_map.offset(self.lowest_idx as isize))._refcount.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        }
        let result = page2virt(self.lowest_idx as u64);
        logk!("kernel get page {:#x}\n", self.lowest_idx);
        self.lowest_idx += 1;
        self.free_pages -= 1;
        unsafe
        {
            relocation::KERNEL_SIZE += PAGE_SIZE
        }
        result
    }
}

fn temporary_alloc_page_frame(num : usize) -> *mut c_void
{
    assert!(num == 1, " temporary alloc only allow 1 page per time");
    unsafe { MEMORY_POOL.alloc_frame_temporary() }
}

#[derive(Clone, Copy)]
struct MemorySpan
{
    size_level : usize,
    prev : *mut MemorySpan,
    next : *mut MemorySpan
}

impl MemorySpan
{
    const fn new() -> MemorySpan
    {
        MemorySpan { size_level: 0, prev: null_mut(), next: null_mut() }
    }
}

pub struct MemoryDescriptor
{
    size : usize,
    all_pages : usize,
    start : *const c_void,
}

#[repr(packed)]
struct E820Map
{
    addr : u64,
    size : u64,
    memory_type : u32
}

impl fmt::Display for E820Map
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mem_type = self.memory_type;
        let addr = self.addr;
        let size = self.size;
        write!(f, "Address: {:#x},\tLength: {:#x},\tType: {:#x}\n", addr, size, mem_type)
    }
}
pub struct Pml4
{
    pub entry : [Pml4Entry; 512]
}

pub type Pdpt = Pml4;
pub type Pdt = Pml4;
struct Pt
{
    entry : [PtEntry; 512]
}
type PdptEntry = Pml4Entry;
type PdtEntry = Pml4Entry;
type PdEntry = Pml4Entry;

bitfield!
{
    pub struct Pml4Entry(u64);
    u64;
    // 0 exist in memory
    get_present, set_present : 0, 0;
    // 0 readonly / 1 read & writable
    get_wr, set_wr : 1, 1;
    // 0 supervisor / 1 everyone
    get_us, set_us : 2, 2;
    // 1 Write Through / 0 Write Back
    get_pwt, set_pwt : 3, 3;
    // page cache disable
    get_pcd, set_pcd : 4, 4;
    // page accessed
    get_accessed, set_accessed : 5, 5;
    // dirty
    get_dirty, set_dirty : 6, 6;
    // page size
    get_ps, set_ps : 7, 7;
    // global
    get_global, set_global : 8, 8;
    // avaliable
    get_avl, set_avl : 11, 9;
    get_page_offset, set_page_offset : 63, 12;
}


impl Display for Pml4Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Present: {};\nWritable: {};\nEveryone: {};\nPWT: {};\nPCD: {};\nAccessed: {};\nDirty: {};\nBig Page: {};\nAVL: {:#x};\nPhyaddr: {:#x};\n", self.get_present() != 0, self.get_wr() != 0,
            self.get_us() != 0, self.get_pwt() != 0, self.get_pcd() != 0, self.get_accessed() != 0, self.get_dirty() != 0, self.get_ps() != 0, self.get_avl(), self.get_page_offset())
    }
}

bitfield!
{
    struct PtEntry(u64);
    u64;
    // 0 exist in memory
    get_present, set_present : 0, 0;
    // 0 readonly / 1 read & writable
    get_wr, set_wr : 1, 1;
    // 0 supervisor / 1 everyone
    get_us, set_us : 2, 2;
    // 1 Write Through / 0 Write Back
    get_pwt, set_pwt : 3, 3;
    // page cache disable
    get_pcd, set_pcd : 4, 4;
    // page accessed
    get_accessed, set_accessed : 5, 5;
    // dirty
    get_dirty, set_dirty : 6, 6;
    // page size
    get_pat, set_pat : 7, 7;
    // global
    get_global, set_global : 8, 8;
    // avaliable
    get_avl, set_avl : 11, 9;
    get_page_offset, set_page_offset : 63, 12;
}

#[inline]
fn get_page_start(addr : *const c_void) -> *const c_void
{
    (addr as u64 & 0xfffffffffffff000) as *const c_void
}

#[inline]
fn get_inpage_offset(ptr : *const c_void) -> usize
{
    (ptr as u64 & 0xfffff).try_into().unwrap()
}

#[inline]
fn get_pdpt_offset(ptr : *const c_void) -> usize
{
    ((ptr as u64 >> 30) & 0x1ff).try_into().unwrap()
}

#[inline]
fn get_pdt_offset(ptr : *const c_void) -> usize
{
    ((ptr as u64 >> 21) & 0x1ff).try_into().unwrap()
}

#[inline]
fn get_pt_offset(ptr : *const c_void) -> usize
{
    ((ptr as u64 >> 12) & 0x1ff).try_into().unwrap()
}

#[inline]
fn get_pml4_offset(ptr : *const c_void) -> usize
{
    ((ptr as u64 >> 39) & 0x1ff).try_into().unwrap()
}

#[inline]
fn set_cr3_reg(pml4_ptr : *const c_void)
{
    unsafe { asm!(
            "mov cr3, {_pml4_ptr}",
            _pml4_ptr = in(reg) pml4_ptr
        ) };
}

#[inline]
fn get_cr3_reg() -> u64
{
    let mut cr3_reg : u64;
    unsafe { asm!("mov {cr3}, cr3",
            cr3 = out(reg) cr3_reg 
        ) };
    cr3_reg
}

fn print_ards(mut e820map_addr : *const E820Map)
{
    unsafe
    {
        loop {
            if (*e820map_addr).memory_type > 4
            {
                break;
            }
            else {
                printk!("{}", (*e820map_addr))
            }
            e820map_addr = e820map_addr.offset(1);
        }
    }

}

unsafe fn link_pages(vaddr : *const c_void, paddr : *const c_void, kernel_space : bool, writable : bool)
{
    let pml4 = phys2virt((get_cr3_reg() & 0xfffffffffffff000) as *const c_void) as *mut Pml4;
    let pm4_offset = get_pml4_offset(vaddr);
    if (*pml4).entry[pm4_offset].get_present() == 0
    {
        let new_pdpt = MEMORY_POOL.alloc_frames(1);
        MemoryPool::set_pml4(pml4, vaddr, virt2phys(new_pdpt), false, kernel_space, true)
    }
    let pdpt = phys2virt(((*pml4).entry[pm4_offset].get_page_offset() << 12) as *const c_void) as *mut Pdpt;
    let pdpt_offset = get_pdpt_offset(vaddr);
    if (*pdpt).entry[pdpt_offset].get_present() == 0
    {
        let new_pdt = MEMORY_POOL.alloc_frames(1);
        MemoryPool::set_pdpt(pdpt, vaddr, virt2phys(new_pdt), false, kernel_space, true);
    }
    let pdt = phys2virt(((*pdpt).entry[pdpt_offset].get_page_offset() << 12) as *const c_void) as *mut Pdt;
    let pdt_offset = get_pdt_offset(vaddr);
    if (*pdt).entry[pdt_offset].get_present() == 0
    {
        let new_pt = MEMORY_POOL.alloc_frames(1);
        MemoryPool::set_pdt(pdt, vaddr, virt2phys(new_pt), false, kernel_space, true);
    }
    let pt = phys2virt(((*pdt).entry[pdt_offset].get_page_offset() << 12) as *const c_void) as *mut Pt;
    let pt_offset = get_pt_offset(vaddr);
    MemoryPool::set_pt(&mut (*pt).entry[pt_offset], paddr, true, writable, !kernel_space, false, false, false, false, false, false)
}

unsafe fn get_useable_memory(descriptor : *const E820Map)
{
    if (*descriptor).size as usize > MEMORY_DESCRIPTOR.size
    {
        MEMORY_DESCRIPTOR.size = (*descriptor).size as usize;
        MEMORY_DESCRIPTOR.start = (*descriptor).addr as *const c_void;
    }
}

fn get_page_size()
{
    unsafe
    {
        MEMORY_DESCRIPTOR.all_pages = MEMORY_DESCRIPTOR.size / PAGE_SIZE;
    }
}

#[inline]
unsafe fn get_kernel_size() -> usize
{
    relocation::KERNEL_SIZE - KERNEL_START
}

fn reset_map_type()
{

}

pub fn init_memory(magic : u32, address : *const c_void)
{
    let mut e820map_addr : *mut E820Map = ARDS_BUFFER as *mut E820Map;
    print_ards(e820map_addr);
    unsafe
    {
        e820map_addr = e820map_addr.offset(1);
        loop {
            match (*e820map_addr).memory_type {
                1 => {
                    get_useable_memory(e820map_addr);
                    e820map_addr = e820map_addr.offset(1);
                    continue;
                },
                2 | 3 => {
                    e820map_addr = e820map_addr.offset(1);
                    continue;
                },
                _ => break
            } 
        }
        get_page_size();
        printk!("total page num: {}\n", MEMORY_DESCRIPTOR.all_pages);
        printk!("kernel size: {}KB\n", get_kernel_size() / 1024);
        MEMORY_POOL.init(&mut MEMORY_DESCRIPTOR);
        set_interrupt_handler(page_fault as interrupt::HandlerFn, interrupt::INTR_PF as u8);
        sched::RUNNING_PROCESS.resize(get_cpu_number(), null_mut());
    }
}

fn link_user_page(vaddr : *const c_void)
{
    unsafe
    {
        let new_page = MEMORY_POOL.alloc_frames(1);
        link_pages(get_page_start(vaddr), virt2phys(new_page), false, true);
    }
}

extern "C" fn page_fault(vector : u64, regs : PtRegs)
{
    unsafe
    {
        assert!(vector == interrupt::INTR_PF);
        let pg_fault_pos = get_cr2_reg();
        let pcb = get_current_running_process();
        if (*pcb).mm.contain(pg_fault_pos as u64)
        {
            link_user_page(pg_fault_pos);
        }
    }

}