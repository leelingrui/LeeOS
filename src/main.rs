#![no_main]
#![feature(ptr_metadata)]
#![feature(int_roundings)]
#![feature(const_trait_impl)]
#![feature(core_intrinsics)]
#![no_std]
use core::arch::global_asm;
use core::arch::asm;
extern crate alloc;

use kernel::{console::console_init, global::gdt_init, interrupt::interrupt_init};
use crate::kernel::global::tss_init;
use crate::kernel::interrupt;
use crate::kernel::process::init;
use crate::kernel::{clock::{self}, memory::init_memory, cpu, fpu::fpu_init};
mod kernel;
// use kernel::console::Console;
global_asm!(include_str!("./kernel/entry.asm"));


#[no_mangle]
unsafe fn kernel_init()
{
    console_init();
    gdt_init();
    interrupt_init();
    init_memory(0, core::ptr::null());
    tss_init();
    fpu_init();
    init();
    printk!("end call int");
}
