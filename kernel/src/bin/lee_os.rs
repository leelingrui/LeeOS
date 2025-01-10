#![no_main]
#![feature(ptr_metadata)]
#![feature(int_roundings)]
#![feature(const_trait_impl)]
#![feature(core_intrinsics)]
#![feature(alloc_layout_extra)]
#![feature(allocator_api)]
#![no_std]
extern crate alloc;
use core::{arch::global_asm, panic::PanicInfo};
use alloc::string::ToString;
use lee_os::{kernel::{clock::clock_init, console::console_init, global::{gdt_init, tss_init}, interrupt::{self, interrupt_init}, process::process_init, ramdisk::ramdisk_init}, mm::{memory::init_memory, shmem::init_shmem}, printk};
use proc_macro::__init;


// use kernel::console::Console;
global_asm!(include_str!("../kernel/entry.asm"));

#[panic_handler]
pub fn panic(_info: &PanicInfo) -> !
{
    printk!("{:#?}\n", _info.to_string());
    loop {
        
    }
}


#[__init]
#[no_mangle]
fn kernel_init()
{
    unsafe
    {
        console_init();
        gdt_init();
        interrupt_init();
        init_memory(0, core::ptr::null());
        ramdisk_init(); 
        init_shmem();
        tss_init();
        clock_init();
        process_init();
        interrupt::set_interrupt_state(true);
        printk!("end call init");
    }
}
