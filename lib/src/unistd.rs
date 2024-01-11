use core::ffi::c_char;

use lee_os::{kernel::{syscall::{__NR_WRITE, __syscall3, __syscall0, __NR_FORK}, process::Pid}, fs};

pub fn write(fd : fs::file::FileDescriptor, buf : *const c_char, count : usize) -> usize
{
    unsafe {
        __syscall3(__NR_WRITE, fd as u64, buf as u64, count as u64)
    }
}

pub fn fork() -> Pid
{
    unsafe
    {
        __syscall0(__NR_FORK) as Pid
    }
}