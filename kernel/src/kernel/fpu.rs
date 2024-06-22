use core::{arch::asm, ptr::null_mut};
use proc_macro::__init;

use crate::{logk, kernel::{cpu::{set_cr0, get_cr0, Cr0RegLabel}, interrupt::HandlerFn, sched}, bochs_break};

use super::{cpu, process::{self, PCB}, interrupt};

static mut LAST_FPU_TASK : *mut process::PCB = null_mut();

fn fpu_handler(vector : u64)
{
    logk!("fpu exception occured\n");
    assert!(vector == interrupt::INTR_NM);
    set_cr0(get_cr0() & !(Cr0RegLabel::CR0_EM.bits() | Cr0RegLabel::CR0_TS.bits()));
    let running_process = sched::get_current_running_process();
    // assert(task->uid);
    fpu_enable();
}

fn fpu_enable()
{
    unsafe
    {
        asm!("fnclex");
        asm!("fninit");
    }

}

#[__init]
fn fpu_check() -> bool
{
    let cpuid = cpu::__cpuid(cpu::EXTENDED_PROCESSOR_SIGNATURE_AND_FEATURE);
    if (cpuid.edx & cpu::FPU_ENABLE) == 0
    {
        return false;
    }
    else
    {
        unsafe
        {
            let ret : u32;

            let test_word = 0x55aau32;
            asm!(
                "mov rdx, cr0",
                "and rdx, rcx",
                "mov cr0, rdx",
                "fninit",
                "fnstsw [{ctrl_word}]",
                "mov rax, [{ctrl_word}]",
                out("rax") ret,
                ctrl_word = in(reg) &test_word as *const u32,
                in("rcx") (0xffffffffffffffff - cpu::Cr0RegLabel::CR0_EM.bits() - cpu::Cr0RegLabel::CR0_TS.bits()),
                out("rdx") _
            );
            ret == 0
        }
    }
}

#[__init]
pub fn fpu_init()
{
    logk!("initial fpu\n");
    let fpu_exist = fpu_check();
    assert!(fpu_exist);
    if fpu_exist
    {
        interrupt::set_interrupt_handler(fpu_handler as interrupt::HandlerFn, interrupt::INTR_NM as u8);
        set_cr0(get_cr0() | (Cr0RegLabel::CR0_EM.bits() | Cr0RegLabel::CR0_TS.bits() | Cr0RegLabel::CR0_NE.bits()) as u64);
    }
}