#![no_main]
#![feature(ptr_metadata)]
#![feature(int_roundings)]
#![feature(const_trait_impl)]
#![feature(core_intrinsics)]
#![feature(asm_const)]
#![no_std]
use core::arch::{global_asm, asm};

use kernel::{console::console_init, global::gdt_init, interupt::interrupt_init};

use crate::kernel::{clock::start_beep, memory::init_memory};
mod kernel;
// use kernel::console::Console;
global_asm!(include_str!("./kernel/entry.asm"));


#[no_mangle]
unsafe fn kernel_init()
{
    console_init();
    gdt_init();
    interrupt_init();
    start_beep();
    init_memory(0, core::ptr::null());
    printk!("end call int");
}
