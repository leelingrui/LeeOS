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
use core::panic::PanicInfo;

use lee_os::kernel::{console::console_init, global::gdt_init, interrupt::interrupt_init};
use lee_os::fs::super_block::super_init;
use lee_os::kernel::clock::clock_init;
use lee_os::kernel::global::tss_init;
use lee_os::kernel::interrupt;
use lee_os::kernel::io::ide_init;
use lee_os::kernel::process::process_init;
use lee_os::kernel::syscall::syscall_init;
use lee_os::kernel::fpu::fpu_init;
use lee_os::mm::memory::init_memory;
use lee_os::printk;

// use kernel::console::Console;
global_asm!(include_str!("../kernel/entry.asm"));

#[panic_handler]
pub fn panic(_info: &PanicInfo) -> !
{
    printk!("{_info}\n");
    loop { }
}



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
