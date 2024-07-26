use core::{ffi::c_void, ptr::{null_mut, null}, sync::atomic::AtomicU32, intrinsics::{log2f64, powf64, ceilf64}};
use alloc::{rc::Rc, string::String};
use crate::{kernel::bitops, bochs_break};
use core::arch::asm;
use crate::kernel::{list::ListHead, semaphore, math};

use super::{page::{GFP, self}, memory::{self, MEMORY_POOL}};
const MAX_NUMNODES : usize = 1;
const KMALLOC_THREASHHOLD : usize = 0x800;
pub const KMALLOC_CACHES_NUM : usize = 12;
pub static mut SLAB_CACHES : *mut KmemCache = null_mut();
macro_rules! init_kmalloc_info {
    ($size: expr, $__short_size: expr) => {
        KMallocInfoStruct::new(concat!("kmalloc-rcl-", stringify!($__short_size)), $size)
    };
}

#[derive(Clone, Copy)]
pub struct KMallocInfoStruct
{
    pub name : &'static str,
    pub size : u32
}

pub static KMALLOC_INFO : [KMallocInfoStruct; KMALLOC_CACHES_NUM] =
[
    init_kmalloc_info!(0, 0),
    init_kmalloc_info!(96, 96),
    init_kmalloc_info!(192, 192),
    init_kmalloc_info!(8, 8),
    init_kmalloc_info!(16, 16),
    init_kmalloc_info!(32, 32),
    init_kmalloc_info!(64, 64),
    init_kmalloc_info!(128, 128),
    init_kmalloc_info!(256, 256),
    init_kmalloc_info!(512, 512),
    init_kmalloc_info!(1024, 1k),
    init_kmalloc_info!(2048, 2k)
]; 

impl KMallocInfoStruct
{
    pub const fn new(name : &'static str, size : u32) -> KMallocInfoStruct
    {
        KMallocInfoStruct { name, size }
    }
}
pub static mut KMALLOC_CACHES : [*mut KmemCache; KMALLOC_CACHES_NUM] = [null_mut(); KMALLOC_CACHES_NUM];

const SIZE_INDEX : [u8; 24] = [
    3,	/* 8 */
	4,	/* 16 */
	5,	/* 24 */
	5,	/* 32 */
	6,	/* 40 */
	6,	/* 48 */
	6,	/* 56 */
	6,	/* 64 */
	1,	/* 72 */
	1,	/* 80 */
	1,	/* 88 */
	1,	/* 96 */
	7,	/* 104 */
	7,	/* 112 */
	7,	/* 120 */
	7,	/* 128 */
	2,	/* 136 */
	2,	/* 144 */
	2,	/* 152 */
	2,	/* 160 */
	2,	/* 168 */
	2,	/* 176 */
	2,	/* 184 */
	2	/* 192 */
];

#[inline]
fn size_index_elem(bytes : usize) -> usize
{
    (bytes - 1) / 8
}


pub fn kmalloc_slab(size : usize,  flags : GFP) -> *mut KmemCache
{
    let index;
    if size <= 192
    {
        if size == 0
        {
            return null_mut();
        }
        index = SIZE_INDEX[size_index_elem(size)];
    }
    else {
        index = bitops::fls64(size as u64) as u8;
    }
    unsafe { KMALLOC_CACHES [index as usize] }
}

struct KmemCacheCpu
{
    freelist : *mut *mut c_void,
    tid : u64,
    page : page::Page,
    partial : *mut page::Page
}

pub struct KmemCacheNode
{
    lock : semaphore::UnreenterabkeSpinLock,
    nr_partial : u64, // number of slub
    partial : ListHead // free slub list
}

#[repr(C)]
pub struct Slab
{
    __page_flags : page::Pageflags,
    slab_cache : *mut KmemCache,
    slab_list : ListHead,
    free_list : *mut c_void,
    __page_refcount : AtomicU32
}

impl Slab {
    fn slab_nid(&mut self) -> usize
    {
        (self.__page_flags.bits() >> page::NODES_WIDTH & page::NODES_MASK) as usize
    }

    fn next(&self) -> *mut Slab
    {
        self.slab_list.next as *mut Slab
    }

    fn set_next(&mut self, next : *mut Slab)
    {
        self.slab_list.next = next as *mut ListHead;
    }

    fn set_prev(&mut self, prev : *mut Slab)
    {
        self.slab_list.prev = prev as *mut ListHead;
    }

    fn prev(&self) -> *mut Slab
    {
        self.slab_list.prev as *mut Slab
    }
}

struct KmemCacheOrderObjects
{
    x : u32
}

impl KmemCacheOrderObjects {
    pub fn new() -> KmemCacheOrderObjects
    {
        KmemCacheOrderObjects { x: 0 }
    }
}

#[repr(C)]
pub struct KmemCache
{
    flags : page::Pageflags,
    min_partial : u64,
    size : u32,
    object_size : u32,
    reciprocal_size : u8,
    pub offset : u32,
    allocflags : page::GFP,
    refcount : u32,
    ctor : *mut extern fn(*mut c_void),
    inuse : u32,
    align : u32,
    red_left_pad : u32,
    name : *const str,
    pub list : ListHead,
    pub node : [KmemCacheNode; MAX_NUMNODES]
}

impl KmemCacheNode {
    fn new() -> KmemCacheNode
    {
        KmemCacheNode { lock: semaphore::UnreenterabkeSpinLock::new(1), nr_partial: 0, partial: ListHead::empty() }
    }

    pub fn kmem_cache_node_bootstrap(&mut self, kmem_cache : &KmemCache)
    {
        if kmem_cache.object_size == 0
        {
            return;
        }
        unsafe
        {
            let mut new_frame = memory::MEMORY_POOL.alloc_frame_temporary();
            let allocable_object_num = memory::PAGE_SIZE / kmem_cache.object_size as usize - 1;
            let page_discriptor = MEMORY_POOL.mem_map.offset(memory::virt2page(new_frame) as isize) as *mut Slab;
            (*page_discriptor).free_list = new_frame;
            (*page_discriptor).__page_flags = kmem_cache.flags;
            let mut var = 0;
            while var < allocable_object_num {
                let next_object = new_frame.offset(kmem_cache.object_size as isize);
                memory::set_free_pointer(kmem_cache, new_frame, next_object);
                new_frame = next_object;
                var += 1;
            }
            self.nr_partial += (allocable_object_num + 1) as u64;
            memory::set_free_pointer(kmem_cache, new_frame, null());
            let next = self.partial.next as *mut Slab;
            (*page_discriptor).slab_list.prev = null_mut();
            if !next.is_null()
            {
                (*next).slab_list.prev = page_discriptor as *mut ListHead;
                (*page_discriptor).slab_list.next = next as *mut ListHead;
            }
            self.partial.next = page_discriptor as *mut ListHead;
        }
    }
    // fn allocate_slab(&mut self, s : &mut KmemCache)
    // {
    //     unsafe
    //     {
    //         let mut slab = Self::allocate_slab_page();
    //         (*slab).slab_cache = s as *mut KmemCache;
    //     }
    // }
}

impl KmemCacheCpu {
    
}

impl KmemCache {
    // allocate object from KmemCacheNode
    fn alloc_node(&mut self, nid : usize) -> *mut c_void
    {
        unsafe
        {
            self.node[nid].lock.acquire(1);
            // have object to allocate
            if self.node[nid].nr_partial > 0
            {
                let mut slab_discriptor = self.node[nid].partial.next as *mut Slab;
                while (*slab_discriptor).free_list.is_null() {
                    slab_discriptor = (*slab_discriptor).next();
                }
                let result = (*slab_discriptor).free_list;
                (*slab_discriptor).free_list = memory::get_free_pointer(self, result);
                self.node[nid].nr_partial -= 1;
                self.node[nid].lock.release(1);
                return result as *mut c_void;
            }
            // no object to allocate
            else {
                self.node[nid].lock.release(1);
                return null_mut();
            }
        }
    }

    pub fn link_to_cache_list(&mut self)
    {
        unsafe
        {
            self.set_next(SLAB_CACHES);
            SLAB_CACHES = self as *mut Self;
        }
    }

    pub fn create_cache(name : *const str, size : u32, align : u32, ctor : *mut extern fn(*mut c_void), flags : page::Pageflags) -> KmemCache
    {
        let mut new_slub = KmemCache { flags, min_partial: 0, size, object_size: 0, reciprocal_size: 0, offset: 0, allocflags: page::GFP::empty(), refcount: 1, ctor, inuse: 0, align, red_left_pad: 0, name, list: ListHead::empty(), node: [KmemCacheNode::new(); MAX_NUMNODES] };
        new_slub.kmem_cache_open(page::GFP::KERNEL);
        new_slub
    }

    fn calculate_size(&mut self)
    {
        if self.size == 0
        {
            return;
        }
        self.object_size = math::upround(self.size as u64, self.align as u64) as u32;
    }

    fn kmem_cache_open(&mut self, flags: page::GFP)
    {
        self.allocflags = flags;
        self.calculate_size();
    }

    fn alloc_single_from_new_slab(&mut self, new_slab : *mut Slab) -> *mut c_void
    {
        unsafe
        {
            let mut var = 0;
            let num_object = memory::PAGE_SIZE / self.object_size as usize - 1;
            let mut object = (*new_slab).free_list;
            // create object list
            while var < num_object {
                let next_object = object.offset(self.object_size as isize);
                memory::set_free_pointer(self, object, next_object);
                object = next_object;
                var += 1;
            }
            memory::set_free_pointer(self, object, null());
            // link to partial list
            let nid = (*new_slab).slab_nid();
            let next = self.node[nid].partial.next as *mut Slab;
            (*next).slab_list.prev = new_slab as *mut ListHead;
            (*new_slab).slab_list.next = next as *mut ListHead;
            self.node[nid].partial.next = new_slab as *mut ListHead;
            self.node[nid].nr_partial += num_object as u64 + 1;
            self.alloc_node(nid)
        }
    }

    pub fn get_next(&mut self) -> *mut KmemCache
    {
        self.list.next as *mut KmemCache
    }

    pub fn get_prev(&mut self) -> *mut KmemCache
    {
        self.list.prev as *mut KmemCache
    }

    pub fn set_next(&mut self, next : *const KmemCache)
    {
        self.list.next = next as *mut ListHead;
    }

    pub fn set_prev(&mut self, prev : *const KmemCache)
    {
        self.list.prev = prev as *mut ListHead;
    }

    fn alloc_from_buddy_system(&mut self, page_num : usize) -> *mut c_void
    {
        unsafe
        {
            let result = memory::MEMORY_POOL.alloc_frames(page_num);
            if result.is_null()
            {
                panic!("out of memory");
            }
            else {
                let mut var = 0;
                while var < page_num {
                    (*memory::MEMORY_POOL.mem_map.offset(var as isize)).flags = self.flags;
                    var += 1;
                }
            }
            result
        }
    }

    pub fn alloc(&mut self) -> *mut c_void
    {
        unsafe
        {
            let mut object = self.alloc_node(0);
            if object.is_null()
            {
                let new_page = self.alloc_from_buddy_system(1);
                let page_discriptor = memory::MEMORY_POOL.mem_map.offset(memory::virt2page(new_page).try_into().unwrap()) as *mut Slab;
                (*page_discriptor).free_list = new_page;
                object = self.alloc_single_from_new_slab(page_discriptor);
            }
            object
        }
    }
}