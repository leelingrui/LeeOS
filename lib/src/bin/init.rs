#![no_main]
#![no_std]
#![feature(start)]
use core::panic::PanicInfo;

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

}