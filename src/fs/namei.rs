use core::ffi::c_char;

#[inline]
fn is_separator(c : c_char) -> bool
{
    c == '\\' as i8 || c == '/' as i8
}

pub fn named(path_name : *mut c_char, next : *mut *mut c_char)
{
    unsafe
    {
        
    }
}

pub fn namei(path : *mut c_char)
{
    unsafe
    {

    }
}