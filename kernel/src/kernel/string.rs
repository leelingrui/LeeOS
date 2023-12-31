use core::{ffi::c_char, ptr::null_mut};

pub const EOS : c_char = 0;

pub unsafe fn memcpy_s<T : Copy>(mut dst : *mut T, dst_size : usize, mut src : *const T, src_size : usize)
{
    let mut cpy_size;
    if dst_size > src_size
    {
        cpy_size = src_size;
    }
    else {
        cpy_size = dst_size;
    }
    while cpy_size > 0 {
        *dst = *src;
        dst = dst.offset(1);
        src = src.offset(1);
        cpy_size -= 1
    }
}

pub unsafe fn memset(mut dst : *mut u8, value : u8, size : usize)
{
    let mut var = 0;
    while var < size {
        *dst = value;
        dst = dst.offset(1);
        var += 1;
    }
}

#[inline]
pub fn is_separator(c : c_char) -> bool
{
    c == '\\' as i8 || c == '/' as i8
}

pub unsafe fn strrsep(str : *const c_char) -> *mut c_char
{
    let mut ptr = str as *mut c_char;
    let mut last = null_mut();
    loop {
        if is_separator(*ptr)
        {
            last = ptr
        }
        if *ptr == EOS
        {
            return last;
        }
        ptr = ptr.offset(1);
    }
}