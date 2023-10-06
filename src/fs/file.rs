use core::{ffi::{c_char, c_void}, alloc::{GlobalAlloc, Layout}};

use crate::{kernel::{console::CONSOLE, io::IdeDiskT, device::{DevT, get_device, device_request, DevReqType}}, mm::memory::{MEMORY_POOL, PAGE_SIZE}};

use super::ext4::Idx;



pub type FileDescriptor = u32;
pub const STDIN : u32 = 0;
pub const STDOUT : u32 = 1;
pub const STDERR : u32 = 2;
pub const EOF : i64 = -1;

#[inline]
fn get_device_buffer(dev : DevT, block : Idx) -> *mut c_void
{
    unsafe
    {
        MEMORY_POOL.alloc(Layout::new::<[c_void; PAGE_SIZE]>()) as *mut c_void
    }
}

pub fn disk_read(dev : DevT, idx : Idx, ) -> *mut c_void
{

    let buffer = get_device_buffer(dev, idx);
    device_request(dev, buffer, 1, idx, 0, DevReqType::Read);
    buffer
}

pub fn sys_write(fd : FileDescriptor, buf : *const c_void, count : usize)
{
    if fd == STDOUT
    {
        unsafe { 
            CONSOLE.write(buf as *const c_char, count);
        }
    }
}