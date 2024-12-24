use core::{ptr::null_mut, ffi::{c_void, c_char}};
use proc_macro::__init;

use crate::{bochs_break, fs::file::sys_write, kernel::{fork::sys_fork, process::{self, sys_yield, sys_exit}, sched::get_current_running_process, syscall_defs::{__NR_FORK, __NR_SCHED_YIELD, __NR_WRITE, __NR_SYS_EXECVE, __NR_EXIT}, execve::sys_execve}, logk};

use super::{cpu, process::PtRegs, interrupt::HANDLER_TABLE};
use core::arch::asm;

pub type SyscallrFn = extern "C" fn();
extern "C"
{
    fn _syscall_start();
} 

#[no_mangle]
pub static mut SYSTEM_CALL_TABLE : [SyscallrFn; 256] = [unsafe { core::mem::transmute::<*mut(), SyscallrFn>(default_syscall as *mut()) }; 256];

pub unsafe fn default_syscall()
{
    logk!("bad syscall");
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

#[__init]
pub fn syscall_init()
{
    // cpu::wrmsr(0x174, 0x8);
    // cpu::wrmsr(0x175, 0xffff800000090000u64);
    // cpu::wrmsr(0x176, _syscall_start as u64);
    logk!("initialating system call\n");
    cpu::wrmsr(0xc0000080, 0x501);
    cpu::wrmsr(0xc0000081, (0x8u64 << 32) | (0x10u64 << 48) as u64);
    cpu::wrmsr(0xc0000082, _syscall_start as u64);
    cpu::wrmsr(0xc0000084, 0x1 << 9);
    // regist syscall to syscall table
    unsafe {
        SYSTEM_CALL_TABLE[__NR_WRITE] = core::mem::transmute::<*mut(), SyscallrFn>(sys_write as *mut());
        SYSTEM_CALL_TABLE[__NR_SCHED_YIELD] = core::mem::transmute::<*mut(), SyscallrFn>(sys_yield as *mut());
        SYSTEM_CALL_TABLE[__NR_FORK] = core::mem::transmute::<*mut(), SyscallrFn>(sys_fork as *mut());
        SYSTEM_CALL_TABLE[__NR_SYS_EXECVE] = core::mem::transmute::<*mut(), SyscallrFn>(sys_execve as *mut());
        SYSTEM_CALL_TABLE[__NR_EXIT] = core::mem::transmute::<*mut(), SyscallrFn>(sys_exit as *mut());
 
    }
}
