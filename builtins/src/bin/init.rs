#![no_main]
#![no_std]
use core::{panic::PanicInfo, ffi::c_char};
use lib::unistd::{self, write, fork};

extern crate builtins;

#[no_mangle]
pub fn main()
{
    let start_str = "init success\n";
    write(0, start_str.as_ptr() as *const i8, start_str.len());
    let pid = fork();
    if pid == 0
    {
        let c_str = "child program\n";
        loop {
            write(0, c_str.as_ptr() as *const c_char, c_str.len());            
        }
    }
    else {
        let p_str = "parent str\n";
        loop {            
            write(0, p_str.as_ptr() as *const c_char, p_str.len());
        }
    }
}
