use core::{ffi::c_void, alloc::{Layout, GlobalAlloc}};

use alloc::alloc::{alloc, dealloc};

use crate::fs::ext4::Idx;

use super::{semaphore::RWLock, device::{DevT, DevReqType, device_request}};

pub struct Buffer
{
    dev : DevT,
    idx : Idx,
    rw_lock : RWLock,
    pub buffer : *mut c_void,
    buffer_size : usize,
    pub count : usize,
    avaliable : bool,
    pub dirty : bool
}

impl Buffer {
    pub fn get_idx(&self) -> Idx
    {
        self.idx
    }

    pub fn set_idx(&mut self, idx : Idx)
    {
        self.idx = idx;
    }

    pub fn get_dev(&self) -> DevT
    {
        self.dev
    }

    pub fn set_dev(&mut self, dev : DevT)
    {
        self.dev = dev;
    }

    pub fn is_avaliable(&self) -> bool
    {
        self.avaliable
    }

    pub fn new(buffer_size : usize) -> Self
    {
        let buffer = unsafe { alloc(Layout::from_size_align(buffer_size, 8).unwrap()) as *mut c_void };
        Self { rw_lock: RWLock::new(), buffer, buffer_size, count: 1, avaliable: false, dirty: false, dev: 0, idx: 0 }
    }

    pub fn dispose(&mut self)
    {
        unsafe
        {
            dealloc(self.buffer as *mut u8, Layout::from_size_align(self.buffer_size, 8).unwrap());
            dealloc(self as *mut Self as *mut u8, Layout::new::<Self>());
        }
    }

    pub fn write_to_buffer(&mut self)
    {
        todo!()
    }

    pub fn read_from_device(&mut self, dev : DevT, idx : Idx, block_num : usize)
    {
        device_request(dev, (*self).buffer, block_num, idx, 0, DevReqType::Read);
        self.avaliable = true;
    }

    pub fn write_to_device(&mut self)
    {
        todo!();
    }

    pub fn read_from_buffer(&mut self, dst : *mut c_void, offset : usize, len : usize)
    {
        if !self.avaliable
        {
            panic!("read unavaliable buffer");
        }
        if len + offset <= self.buffer_size
        {
            self.rw_lock.rdlock(); 
            unsafe { compiler_builtins::mem::memcpy(dst as *mut u8, self.buffer.offset(offset as isize) as *mut u8, len) };
            self.rw_lock.rdunlock();
        }
        else {
            panic!("read area out of range");
        }
    }

}