use core::{ptr::null_mut, ffi::{c_void, c_char}};
use crate::{fs::file::{self, sys_write}, bochs_break, logk};

use super::{cpu, console::CONSOLE};
use core::arch::asm;

pub type SyscallrFn = *mut extern "C" fn();
extern "C"
{
    fn _syscall_start();
} 

pub const __NR_READ : usize = 0;
pub const __NR_WRITE : usize = 1;


#[no_mangle]
pub static mut SYSTEM_CALL_TABLE : [SyscallrFn; 256] = [null_mut(); 256];

#[inline]
pub unsafe fn __syscall0(nr : usize) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        lateout("rax") result
    );
    result
}

#[inline]
pub unsafe fn __syscall1(nr : usize, arg1 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        lateout("rax") result
    );
    result
}

#[inline]
pub unsafe fn __syscall2(nr : usize, arg1 : u64, arg2 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        in("rsi") arg2,
        lateout("rax") result
    );
    result
}

#[inline]
pub unsafe fn __syscall3(nr : usize, arg1 : u64, arg2 : u64, arg3 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        lateout("rax") result
    );
    result
}

pub fn syscall_init()
{
    // cpu::wrmsr(0x174, 0x8);
    // cpu::wrmsr(0x175, 0xffff800000090000u64);
    // cpu::wrmsr(0x176, _syscall_start as u64);
    logk!("initialating system call\n");
    cpu::wrmsr(0xc0000081, (0x8u64 << 32) | (0x2bu64 << 48) as u64);
    cpu::wrmsr(0xc0000082, _syscall_start as u64);
    cpu::wrmsr(0xc0000080, 0x501);
    unsafe {
        SYSTEM_CALL_TABLE[__NR_WRITE] = sys_write as SyscallrFn;
    }

}