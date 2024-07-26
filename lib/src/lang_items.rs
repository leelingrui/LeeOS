use crate::unistd::exit;
#[linkage = "weak"]
#[no_mangle]
fn main()
{
    panic!("no main() linked");
}

#[start]
#[no_mangle]
fn _start(argc : isize, argv : *const *const u8) -> isize
{
    unsafe
    {
        main();
        exit(0);
        0
    }
}


#[panic_handler]
pub fn panic(_info: &PanicInfo) -> !
{
    println!("{_info}\n");
    loop {
            
        }
}
