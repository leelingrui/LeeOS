#[macro_export]
macro_rules! bit {
	($nr:expr) => {
		1 << $nr
	};
}



#[inline(always)]
pub fn fls(mut x : u32) -> u32
{
	let mut r = 32u32;

	if x == 0
    {
		return 0;
    }
	if x & 0xffff0000 == 0 {
		x <<= 16;
		r = r.checked_sub(16).unwrap_or(r);
	}
	if x & 0xff000000 == 0 {
		x <<= 8;
		r = r.checked_sub(8).unwrap_or(r);
	}
	if x & 0xf0000000 == 0 {
		x <<= 4;
		r = r.checked_sub(4).unwrap_or(r);
	}
	if x & 0xc0000000 == 0 {
		x <<= 2;
		r = r.checked_sub(2).unwrap_or(r);
	}
	if x & 0x80000000 == 0 {
		x <<= 1;
		r = r.checked_sub(1).unwrap_or(r);
	}
	return r;
}

#[inline(always)]
pub fn fls64(mut x : u64) -> u64
{
	let mut r: u64 = 32;
	if (x & 0xffffffff) == 0 {
		r = r.checked_sub(32).unwrap_or(r);
		x <<= 32;
	}

	if x == 0
    {
		return 0;
    }
	if x & 0xffff0000 == 0 {
		x <<= 16;
		r = r.checked_sub(16).unwrap_or(r);
	}
	if x & 0xff000000 == 0 {
		x <<= 8;
		r = r.checked_sub(8).unwrap_or(r);
	}
	if x & 0xf0000000 == 0 {
		x <<= 4;
		r = r.checked_sub(4).unwrap_or(r);
	}
	if x & 0xc0000000 == 0 {
		x <<= 2;
		r = r.checked_sub(2).unwrap_or(r);
	}
	if x & 0x80000000 == 0 {
		x <<= 1;
		r = r.checked_sub(1).unwrap_or(r);
	}
	return r;
}

#[inline(always)]
pub fn ffs(mut x : i32) -> u32
{
	let mut r = 1;

	if x == 0
    {
		return 0;
    }
	if (x & 0xffff) == 0 {
		x >>= 16;
		r += 16;
	}
	if (x & 0xff) == 0 {
		x >>= 8;
		r += 8;
	}
	if (x & 0xf) == 0 {
		x >>= 4;
		r += 4;
	}
	if (x & 3) == 0 {
		x >>= 2;
		r += 2;
	}
	if (x & 1) == 0 {
		x >>= 1;
		r += 1;
	}
	return r;
}

#[inline(always)]
pub fn ffs64(mut x : i64) -> u64
{
	let mut r = 1;
	if (x & 0xffffffff) == 0 {
		r += 32;
		x >>= 32;
	}
	if x == 0
    {
		return 0;
    }
	if (x & 0xffff) == 0 {
		x >>= 16;
		r += 16;
	}
	if (x & 0xff) == 0 {
		x >>= 8;
		r += 8;
	}
	if (x & 0xf) == 0 {
		x >>= 4;
		r += 4;
	}
	if (x & 3) == 0 {
		x >>= 2;
		r += 2;
	}
	if (x & 1) == 0 {
		x >>= 1;
		r += 1;
	}
	return r;
}

