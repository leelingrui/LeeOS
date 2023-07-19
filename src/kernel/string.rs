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