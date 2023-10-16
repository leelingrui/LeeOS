use core::arch::asm;
use core::ops::Add;
use core::ops::Mul;
use core::ops::Div;

use crate::bochs_break;
// pub fn upround<T>(num : T, rank : T) -> T where T: Div<T> + Mul<T> + Add<T>, <T as Div>::Output: Mul<T>
// {
//     (num / rank) * rank
// }

#[no_mangle]
pub fn log2(mut x : f64) -> f64
{
    unsafe {
        asm!(
            "fld1",
            "fld qword ptr [{input}]",
            "fyl2x",
            "fwait",
            "fstp qword ptr [{input}]",
            input = in(reg) &mut x as *mut f64,
        );
        x
    }
}

#[no_mangle]
pub fn ceil(mut x : f64)
{
    unsafe
    {
        let mut temp1 : u16 = 0;
        let mut temp2 : u16;
        asm!(
            "fnstcw [{temp1_input}]",
            temp1_input = in(reg) &mut temp1 as *mut u16
        );
        temp2 = (temp1 & 0xf3ff) | 0x800;
        asm!(
            "fldcw [{temp2_input}]",
            "fld qword ptr [{input}]",
            "frndint",
            "fstp qword ptr [{input}]",
            temp2_input = in(reg) &mut temp2 as *mut u16,
            input = in(reg) &mut x as *mut f64,
        );
        asm!(
            "fldcw [{temp1_input}]",
            temp1_input = in(reg) &mut temp1 as *mut u16
        );
    }
}

pub fn upround(x : u64, round : u64) -> u64
{
    ((x % round != 0) as u64 + x / round) * round
}

#[no_mangle]
pub fn pow(mut x : f64, y : f64) -> f64
{
    unsafe {
        x = log2(x);
        asm!(
            "fld1",
            "fld qword ptr [{inputy}]",
            "fld qword ptr [{inputx}]",
            "fmul",
            "fscale",
            "fstp qword ptr [{inputx}]",
            inputy = in(reg) &y as *const f64,
            inputx = in(reg) &mut x as *mut f64,
        );
        x
    }
}