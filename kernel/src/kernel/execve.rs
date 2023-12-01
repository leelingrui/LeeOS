use core::{ffi::c_char, alloc::Layout};

use crate::mm::memory;

use super::process::PtRegs;

pub fn sys_execve(filename : *const c_char, argv : *mut *mut c_char, envp : *mut *mut c_char)
{
    unsafe
    {
        do_execve(filename, argv, envp);
    }
}

unsafe fn do_execve(name : *const c_char, argv : *mut *mut c_char, envp : *mut *mut c_char) -> u64
{
    // let dst = alloc::alloc::alloc(Layout::for_value(&1024));
    // (*regs).rip = dst as u64;
    // (*regs).rsp = 0xa00000;
    // (*regs).rax = 0x1;
    // (*regs).ds = 0;
    // (*regs).es = 0;
    // compiler_builtins::mem::memcpy(dst as *mut u8, init_thread as *mut u8, 1024);
    return 0;
}