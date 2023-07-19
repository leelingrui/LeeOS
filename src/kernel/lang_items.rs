use core::panic::PanicInfo;

use crate::printk;

#[panic_handler]
pub fn panic(_info: &PanicInfo) -> !
{
    printk!("{_info}\n");
    loop { }
}
