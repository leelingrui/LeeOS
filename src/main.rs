#![no_main]
#![feature(ptr_metadata)]
#![feature(int_roundings)]
#![feature(const_trait_impl)]
#![feature(core_intrinsics)]
#![feature(alloc_layout_extra)]
#![feature(allocator_api)]
#![no_std]
use core::arch::global_asm;
extern crate alloc;

use kernel::{console::console_init, global::gdt_init, interrupt::interrupt_init};
use crate::fs::super_block::super_init;
use crate::kernel::clock::clock_init;
use crate::kernel::global::tss_init;
use crate::kernel::interrupt;
use crate::kernel::io::ide_init;
use crate::kernel::process::process_init;
use crate::kernel::syscall::syscall_init;
use crate::kernel::fpu::fpu_init;
use crate::mm::memory::init_memory;
mod kernel;
mod fs;
mod clib;
mod mm;
// use kernel::console::Console;
global_asm!(include_str!("./kernel/entry.asm"));


#[no_mangle]
unsafe fn kernel_init()
{
    console_init();
    gdt_init();
    interrupt_init();
    init_memory(0, core::ptr::null());
    ide_init();
    tss_init();
    fpu_init();
    syscall_init();
    super_init();
    clock_init();
    process_init();
    interrupt::set_interrupt_state(true);
    printk!("end call int");
}
