use core::panic::PanicInfo;

use crate::printk;

#[macro_export]
macro_rules! bochs_break {
    () => {
        asm!(
            "xchg bx, bx"
        )
    };
}