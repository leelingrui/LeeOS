use core::ffi::c_char;
<<<<<<< HEAD
use lee_os::{kernel::{syscall::{__NR_WRITE, __syscall3, __syscall1, __syscall0, __NR_FORK, __NR_EXIT, __NR_SYS_EXECVE}, process::Pid, Err}, fs};

pub fn write(fd : fs::file::FileDescriptor, buf : *const c_char, count : usize) -> usize
=======
use crate::syscall_defs::{self, __syscall0, __syscall3};

pub fn write(fd : u32, buf : *const c_char, count : usize) -> usize
>>>>>>> mount
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

pub fn exit(error_code : i64)
{
    unsafe
    {
        __syscall1(__NR_EXIT, error_code as u64);
    }
}

pub fn execve(filename : *const c_char, argv : *const *const c_char, envp : *const *const c_char) -> Err
{
    unsafe
    {
        __syscall3(__NR_SYS_EXECVE, filename as u64, argv as u64, envp as u64) as Err
    }
}

