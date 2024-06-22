use core::{ffi::c_char, fmt::{self, Error, Write}};

use crate::unistd::write;

struct Stdout;

impl Stdout
{

}

impl fmt::Write for Stdout
{
    fn write_str(&mut self, output_string : &str) -> fmt::Result
    {
        let ret = write(1, output_string.as_ptr() as *const c_char, output_string.len());
        if ret == output_string.len()
        {
            Ok(())
        }
        else {
            Err(Error)
        }
    }
}

pub fn __print(args : fmt::Arguments)
{
    Stdout{}.write_fmt(args).unwrap()
}