#![no_main]
#![no_std]
#![feature(start)]
use core::panic::PanicInfo;
use lee_os::fs::file::STDOUT;
use lib::unistd::{self, write};
#[panic_handler]
pub fn panic(_info: &PanicInfo) -> !
{
    // printk!("{_info}\n");
    loop { }
}


#[start]
#[no_mangle]
extern "C" fn _start()
{
    main();
    loop{};
}

fn main()
{
    let start_str = "init success";
    write(STDOUT, start_str.as_ptr() as *const i8, start_str.len());
}