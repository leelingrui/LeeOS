use core::ffi::c_char;
use crate::syscall_defs::{self, __syscall0, __syscall3};

pub fn write(fd : u32, buf : *const c_char, count : usize) -> usize
{
    unsafe {
        __syscall3(syscall_defs::__NR_WRITE, fd as u64, buf as u64, count as u64)
    }
}

pub fn fork() -> i32
{
    unsafe
    {
        __syscall0(syscall_defs::__NR_FORK) as i32
    }
}