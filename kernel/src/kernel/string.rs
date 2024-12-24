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

pub unsafe fn strsep(stringp : *mut *mut c_char, delim : *const c_char) -> *mut c_char
{
    let mut ptr = *stringp;
    let first = *stringp;
    loop {
        let mut d_p = delim;
        while *d_p != '\0' as c_char
        {
            if *d_p == *ptr
            {
                *ptr = 0 as c_char;
                *stringp = ptr.offset(1);
                if **stringp == EOS
                {
                    *stringp = null_mut();
                }
                return first;
            }
            d_p = d_p.offset(1);
        }
        if *ptr == EOS
        {
            *stringp = null_mut();
            return first;
        }
        ptr = ptr.offset(1);
    }
}

pub unsafe fn strchr(mut __s : *const c_char, __c : c_char) -> *mut c_char
{
    loop {
        if *__s == __c
        {
            break;
        }
        if *__s == '\0' as c_char
        {
            return null_mut();
        }
        __s = __s.offset(1);
    }
    return  __s as *mut c_char;
}
