use core::arch::x86_64::_mm_crc32_u32;
use core::ffi::c_void;
use core::intrinsics::unlikely;

use crate::__be32_to_cpu;
use crate::__cpu_to_be32;
use crate::__cpu_to_le32;
use crate::__le32_to_cpu;
use crate::__constant_swap32;
use crate::kernel::cpu::__cpuid;
use crate::kernel::cpu::CpuVersion;

use super::crc32table::CRC32C_TABLE_LE;

static mut SSE4_2_ENABLE : bool = false;
pub fn init_crc32()
{
    unsafe
    {
        let cpu_feature = __cpuid(1);
        if CpuVersion::from_bits(cpu_feature.ecx).unwrap().contains(CpuVersion::ECX_SSE4_2)
        {
            SSE4_2_ENABLE = true;
        }
    }
}

pub fn reverse8(data : u8) -> u8 {
    let mut i = 0;
    let mut temp = 0;
    while i < 8    				// 8 bit反转
    {
        temp |= ((data >> i) & 0x01) << (7 - i);
        i += 1;
    }
    return temp;
}

pub fn reverse32(data : u32) -> u32 {
    let mut i = 0;
    let mut temp = 0;
    while i < 32					// 32 bit反转
    {
        temp |= ((data >> i) & 0x01) << (31 - i);
        i += 1;
    }
    return temp;
}

pub fn crc32_body(mut crc : u32, mut buf : *const u8, mut len : usize, table : &[[u32; 256]; 8]) -> u32
{
    unsafe
    {
        while unlikely((len & buf as usize & 3) != 0) {
            crc = (crc >> 8) ^ (table[0][((crc ^ (*buf) as u32) & 255) as usize]);				//与crc初始值高8位异或 
            buf = buf.offset(1);
            len -= 1;
        }
        let rem_len = len & 7;
        len = len >> 3;
        let mut b = buf as *const u32;
        b = b.offset(-1);
        let mut var = 0;
        let mut q;
        while var < len {
            b = b.offset(1);
            q = crc ^ (*b);
            crc = table[7][(q & 255) as usize] ^ table[6][((q >> 8) & 255) as usize] ^ table[5][((q >> 16) & 255) as usize] ^ table[4][((q >> 24) & 255) as usize];
            b = b.offset(1);
            q = *b;
            crc ^= table[3][(q & 255) as usize] ^ table[2][((q >> 8) & 255) as usize] ^ table[1][((q >> 16) & 255) as usize] ^ table[0][((q >> 24) & 255) as usize];
            var += 1;
        }
        len = rem_len;
        if len != 0
        {
            var = 0;
            let mut p = (b.offset(1) as *const u8).offset(-1);
            while var < len {
                p = p.offset(1);
                crc = (crc >> 8) ^ (table[0][((crc ^ (*p) as u32) & 255) as usize]);				//与crc初始值高8位异或 
                len -= 1;
            }
        }
        crc                                 //返回最终校验值
    }
}

pub fn crc32c_le_generic(mut crc : u32, buf : *const u8, bitlen : usize, tab : &[[u32; 256]; 8]) -> u32
{
    unsafe
    {
        if SSE4_2_ENABLE
        {
            unimplemented!();
        }
        else {
            crc = __cpu_to_le32!(crc);
            crc = crc32_body(crc, buf, bitlen, tab);
            crc = __le32_to_cpu!(crc);
            crc
        }

    }
}

pub fn crc32c_le(crc : u32, buf : *const c_void, len : usize) -> u32
{
    crc32c_le_generic(crc, buf as *const u8, len, &CRC32C_TABLE_LE)
}

fn crc32c_be_generic(mut crc : u32, buf : *const u8, len : usize, tab : &[[u32; 256]; 8]) -> u32
{
    unsafe
    {
        if SSE4_2_ENABLE
        {
            unimplemented!();
        }
        else {
            crc = __cpu_to_be32!(crc);
            crc = crc32_body(crc, buf, len, tab);
            crc = __be32_to_cpu!(crc);
            crc
        }

    }
}