use crate::{unistd::exit, println};
use core::panic::PanicInfo;

#[linkage = "weak"]
#[panic_handler]
pub fn panic(_info: &PanicInfo) -> !
{
    println!("{_info}\n");
    loop {
            
        }
}

