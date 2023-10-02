use core::ffi::{c_char, c_void};

use crate::kernel::console::CONSOLE;



pub type FileDescriptor = u32;
pub const STDIN : u32 = 0;
pub const STDOUT : u32 = 1;
pub const STDERR : u32 = 2;
pub const EOF : i64 = -1;

pub fn sys_write(fd : FileDescriptor, buf : *const c_void, count : usize)
{
    if fd == STDOUT
    {
        unsafe { 
            CONSOLE.write(buf as *const c_char, count);
        }
    }
}