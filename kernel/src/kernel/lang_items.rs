use core::panic::PanicInfo;

use crate::printk;

#[panic_handler]
pub fn panic(_info: &PanicInfo) -> !
{
    printk!("{_info}\n");
    loop { }
}

#[macro_export]
macro_rules! bochs_break {
    () => {
        asm!(
            "xchg bx, bx"
        )
    };
}