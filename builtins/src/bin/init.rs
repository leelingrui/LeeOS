#![no_main]
#![no_std]
use core::ffi::c_char;
use lib::unistd::{write, fork};

extern crate builtins;

#[no_mangle]
pub fn main()
{
    let start_str = "init success\n";
    write(1, start_str.as_ptr() as *const i8, start_str.len());
    let pid = fork();
    if pid == 0
    {
        let c_str = "child program\n";
        loop {
            write(1, c_str.as_ptr() as *const c_char, c_str.len());            
        }
    }
    else {
        let p_str = "parent str\n";
        loop {            
            write(1, p_str.as_ptr() as *const c_char, p_str.len());
        }
    }
}
