use core::{arch::asm, ffi::c_void};
use crate::logk;

use super::cpu;
use bitflags::bitflags;
pub const SUPPORT_1GB_PAGE : u32 = 1 << 26;
pub const FPU_ENABLE : u32 = 1 << 0;
pub const GET_CPU_VENDOR_ID : u32 = 0;
pub const GET_CPU_VERSION : u32 = 1;
pub const EXTENDED_PROCESSOR_SIGNATURE_AND_FEATURE : u32 = 0x80000001;

bitflags!
{
    pub struct Cr0RegLabel : i32
    {
        const CR0_PE = 1 << 0;// Protection Enable 启用保护模式
        const CR0_MP = 1 << 1;  // Monitor Coprocessor
        const CR0_EM = 1 << 2;  // Emulation 启用模拟，表示没有 FPU
        const CR0_TS = 1 << 3;  // Task Switch 任务切换，延迟保存浮点环境
        const CR0_ET = 1 << 3;  // Extension Type 保留
        const CR0_NE = 1 << 5;  // Numeric Error 启用内部浮点错误报告
        const CR0_WP = 1 << 16; // Write Protect 写保护（禁止超级用户写入只读页）帮助写时复制
        const CR0_AM = 1 << 18; // Alignment Mask 对齐掩码
        const CR0_NW = 1 << 29; // Not Write-Through 不是直写
        const CR0_CD = 1 << 30; // Cache Disable 禁用内存缓冲
        const CR0_PG = 1 << 31; // Paging 启用分页
    }
}

pub fn get_cpu_number() -> usize
{
    1
}

#[inline]
pub  fn wrmsr(dst : u64, value : u64)
{
    unsafe
    {
        asm!(
            "wrmsr",
            in("rdx") (value >> 32),
            in("rax") value & 0xffffffff,
            in("rcx") dst,
        )
    }
}

#[inline]
pub fn get_cr2_reg() -> *const c_void
{
    let cr2;
    unsafe
    {
        asm!(
            "mov rax, cr2",
            out("rax") cr2
        );
    }

    cr2
}

pub fn cpu_check_cpuid() -> bool
{
    let ret: u64;
    unsafe { asm!(
            "pushfq", // 保存 eflags
            "pushfq",                   // 得到 eflags
            "xor qword ptr [rsp], 0x00200000", // 反转 ID 位
            "popfq",                     // 写入 eflags
            "pushfq",                  // 得到 eflags
            "pop rax",              // 写入 eax
            "xor rax, [rsp]",     // 将写入的值与原值比较    
            "popfq", // 恢复 eflags
            out("rax") ret
        ) };
    return (ret & 0x00200000) != 0;
}
#[derive(Default)]
#[repr(C)]
pub struct CpuidResult
{
    pub eax : u32,
    pub ebx : u32,
    pub ecx : u32,
    pub edx : u32
}

pub fn get_cr0() -> u64
{
    let result;
    unsafe { asm!("mov rax, cr0\n", out("rax") result) };
    result
}

pub fn set_cr0(cr0 : u64)
{
    unsafe
    {
        asm!("mov cr0, rax", in("rax") cr0);
    }
}

// pub fn fpu_enable()
// {
//     set_cr0(get_cr0() & !(Cr0RegLabel::CR0_EM.bits() | Cr0RegLabel::CR0_TS.bits()) as u64);
// }

pub fn __cpuid(selector : u32) -> CpuidResult
{
    let mut result: CpuidResult = Default::default();
    unsafe {
        asm!(
            "push rbx",
            "cpuid",
            "mov rdi, rbx",
            "pop rbx",
            in("eax") selector,
            lateout("edx") result.edx,
            lateout("ecx") result.ecx,
            lateout("edi") result.ebx,
            lateout("eax") result.eax
        );
        result
    }
}