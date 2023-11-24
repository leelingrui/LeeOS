use core::{ffi::c_void, ptr::{null, null_mut}, ops::IndexMut};

use super::string::memset;

pub struct BitMap
{
    data : *mut u8,
    length : usize
}

impl BitMap {
    pub fn new(start_pos : *mut u8, size : usize) -> BitMap
    {
        let bitmap = BitMap { data: start_pos, length: size };
        unsafe { memset(bitmap.data, 0, bitmap.length / 8 + (bitmap.length % 8 != 0) as usize) };
        bitmap
    }

    pub const fn null_bitmap() -> BitMap
    {
        let bitmap = BitMap { data: null_mut(), length: 0 };
        bitmap
    }

    pub fn reset_bitmap(&mut self, start_pos : *mut u8, size : usize)
    {
        self.data = start_pos;
        self.length = size;
        unsafe { memset(self.data, 0, self.length / 8 + (self.length % 8 != 0) as usize) };
    }

    pub fn set(&mut self, mut idx : usize, length : usize, value : bool)
    {
        unsafe
        {
            if idx + length > self.length
            {
                panic!("outof bitmap range");
            }
            let mut var = 0;
            if value == true
            {
                while var < length {
                    (*self.data.offset((idx / 8) as isize)) |= 1 << idx % 8;
                    idx += 1;
                    var += 1;
                }
            }
            else
            {
                while var < length {
                    (*self.data.offset((idx / 8) as isize)) &= !(1 << idx % 8);
                    idx += 1;
                    var += 1;
                }
            }
        }
    }

    pub fn at(&mut self, idx : usize) -> bool
    {
        unsafe
        {
            if idx > self.length
            {
                panic!("outof bitmap range");
            }
            (*self.data.offset((idx / 8) as isize)) & (1 << idx % 8) != 0
        }
    }

    pub fn test_and_set(&mut self, idx : usize) -> bool
    {
        unsafe
        {
            if idx > self.length
            {
                panic!("outof bitmap range");
            }
            let result = (*self.data.offset((idx / 8) as isize)) & (1 << idx % 8) != 0;
            (*self.data.offset((idx / 8) as isize)) |= 1 << idx % 8;
            result
        }
    }
}