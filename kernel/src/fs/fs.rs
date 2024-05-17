use core::{alloc::Layout, ffi::c_void, mem::ManuallyDrop, ptr::{self, null_mut}};

use alloc::collections::BTreeMap;

use crate::{kernel::{buffer::Buffer, semaphore::RWLock}, mm::page::Pageflags};

use super::{ext4::Idx, file::FileFlag, inode::Inode};

pub struct AddressSpace
{
    host : *const Inode,
    i_pages : BTreeMap<Idx, *mut c_void>,
    invalidate_lock : RWLock,
    fgp_mask : Pageflags,
    flags : FileFlag,
    
}

impl AddressSpace
{
    pub fn new(host : *const Inode, fgp_mask : Pageflags, flags : FileFlag) -> *mut Self
    {
        unsafe
        {
            let ptr = alloc::alloc::alloc(Layout::new::<Self>()) as *mut Self;
            *ptr = Self { host, i_pages: BTreeMap::<Idx, *mut c_void>::new(), invalidate_lock: RWLock::new(), fgp_mask, flags };
            ptr
        }
    }

    pub fn destory(&mut self)
    {
        unsafe
        {
            ptr::drop_in_place(self);
            alloc::alloc::dealloc(self as *mut Self as *mut u8, Layout::new::<Self>());
        }
    }

    pub fn seek(&self, idx : Idx) -> *mut c_void
    {
        match self.i_pages.get(&idx) {
            Some(page) => *page,
            None => null_mut(),
        }
    }
}