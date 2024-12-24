use lib::{unistd::exit, println};
use core::panic::PanicInfo;

#[linkage = "weak"]
#[no_mangle]
fn main()
{
    panic!("no main() linked");
}

#[no_mangle]
pub extern "C" fn _start(_argc : isize, _argv : *const *const u8) -> !
{
    unsafe
    {
        main();
        exit(0);
    }
}


// #[panic_handler]
pub fn panic(_info: &PanicInfo) -> !
{
    println!("{_info}\n");
    loop {
            
        }
}

