#![no_main]
#![no_std]
#![feature(start)]
use core::{panic::PanicInfo, ffi::c_char};
use lee_os::fs::file::STDOUT;
use lib::unistd::{self, write, fork, exit};
#[panic_handler]
pub fn panic(_info: &PanicInfo) -> !
{
    // printk!("{_info}\n");
    exit(-1);
    loop { }
}

fn main()
{
    let start_str = "init success\n";
    write(STDOUT, start_str.as_ptr() as *const i8, start_str.len());
    let pid = fork();
    if pid == 0
    {
        let c_str = "child program\n";
        loop {
            write(STDOUT, c_str.as_ptr() as *const c_char, c_str.len());            
        }
    }
    else {
        let p_str = "parent str\n";
        loop {            
            write(STDOUT, p_str.as_ptr() as *const c_char, p_str.len());
        }
    }
}
