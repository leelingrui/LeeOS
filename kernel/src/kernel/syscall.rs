use core::{ptr::null_mut, ffi::{c_void, c_char}};
use crate::{fs::file::sys_write, logk, kernel::{process::{self, sys_yield}, fork::sys_fork, sched::get_current_running_process}, bochs_break};

use super::{cpu, process::PtRegs, interrupt::HANDLER_TABLE};
use core::arch::asm;

pub type SyscallrFn = extern "C" fn();
extern "C"
{
    fn _syscall_start();
} 

pub const __NR_READ : usize = 0;
pub const __NR_WRITE : usize = 1;
pub const __NR_SCHED_YIELD : usize = 24;
pub const __NR_FORK : usize = 57;
#[no_mangle]
pub static mut SYSTEM_CALL_TABLE : [SyscallrFn; 256] = [unsafe { core::mem::transmute::<*mut(), SyscallrFn>(default_syscall as *mut()) }; 256];

#[inline(always)]
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

#[inline(always)]
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

#[inline(always)]
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

#[inline(always)]
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

pub unsafe fn default_syscall()
{
    logk!("bad syscall");
}

#[inline(always)]
pub unsafe fn __syscall4(nr : usize, arg1 : u64, arg2 : u64, arg3 : u64, arg4 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        lateout("rax") result
    );
    result
}

#[inline(always)]
pub unsafe fn __syscall5(nr : usize, arg1 : u64, arg2 : u64, arg3 : u64, arg4 : u64, arg5 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        lateout("rax") result
    );
    result
}

#[inline(always)]
pub unsafe fn __syscall6(nr : usize, arg1 : u64, arg2 : u64, arg3 : u64, arg4 : u64, arg5 : u64, arg6 : u64) -> usize
{
    let result;
    asm!(
        "syscall",
        in("rax") nr,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        in("r9") arg6,
        lateout("rax") result
    );
    result
}

pub unsafe fn set_syscall_return_value(ret : u64)
{
    let pcb = get_current_running_process();
    (*(*pcb).get_intr_frame()).rax = ret;
}

#[no_mangle]
pub unsafe fn syscall_function(pt_regs : PtRegs)
{
    let pcb = get_current_running_process();
    let result;
    asm!(
        "mov [{pcb_stack}], rsp",
        "mov rsp, {kernel_stack}",
        kernel_stack = in(reg) (*pcb).get_process_kernel_stack(),
        pcb_stack = in(reg) &(*pcb).stack as *const *mut c_void
    );
    bochs_break!();
    asm!(
        "mov rcx, [SYSTEM_CALL_TABLE@GOTPCREL + rip]",
        "call [rcx + 8 * rax]",
        in("rdi") pt_regs.rdi,
        in("rsi") pt_regs.rsi,
        in("rdx") pt_regs.rdx,
        in("r10") pt_regs.r10,
        in("r8") pt_regs.r8,
        in("r9") pt_regs.r9,
        in("rax") pt_regs.rax,
        lateout("rax") result
    );
    set_syscall_return_value(result);
    let tmp_pcb = get_current_running_process();
    asm!(
        "mov rsp, {restore_stack}",
        restore_stack = in(reg) (*tmp_pcb).stack
    );
}

pub fn syscall_init()
{
    // cpu::wrmsr(0x174, 0x8);
    // cpu::wrmsr(0x175, 0xffff800000090000u64);
    // cpu::wrmsr(0x176, _syscall_start as u64);
    logk!("initialating system call\n");
    cpu::wrmsr(0xc0000081, (0x8u64 << 32) | (0x10u64 << 48) as u64);
    cpu::wrmsr(0xc0000082, _syscall_start as u64);
    cpu::wrmsr(0xc0000080, 0x501);
    // regist syscall to syscall table
    unsafe {
        SYSTEM_CALL_TABLE[__NR_WRITE] = core::mem::transmute::<*mut(), SyscallrFn>(sys_write as *mut());
        SYSTEM_CALL_TABLE[__NR_SCHED_YIELD] = core::mem::transmute::<*mut(), SyscallrFn>(sys_yield as *mut());
        SYSTEM_CALL_TABLE[__NR_FORK] = core::mem::transmute::<*mut(), SyscallrFn>(sys_fork as *mut());
    }

}