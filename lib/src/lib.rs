#![no_std]

use core::panic::PanicInfo;
use core::ffi::c_char;
pub mod unistd;
use crate::unistd::write;
pub mod syscall_defs;
pub mod macros;
pub mod print;





#[panic_handler]
pub fn panic(_info: &PanicInfo) -> !
{
    println!("{_info}\n");
    loop {
            
        }
}