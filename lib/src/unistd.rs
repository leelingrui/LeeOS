use core::ffi::c_char;
use crate::{syscall_defs::{self, __syscall0, __syscall1,  __syscall3}, println};

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

pub fn exit(error_code : i64) -> !
{
    unsafe
    {
        __syscall1(syscall_defs::__NR_EXIT, error_code as u64);
        println!("BUG!!!EXIT ERROR!");
        loop { }
    }
}

pub fn execve(filename : *const c_char, argv : *const *const c_char, envp : *const *const c_char) -> i64
{
    unsafe
    {
        __syscall3(syscall_defs::__NR_SYS_EXECVE, filename as u64, argv as u64, envp as u64) as i64
    }
}

