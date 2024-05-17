use core::{arch::asm, ffi::c_void};

use bitflags::bitflags;
pub const SUPPORT_1GB_PAGE : u32 = 1 << 26;
pub const FPU_ENABLE : u32 = 1 << 0;
pub const GET_CPU_VENDOR_ID : u32 = 0;
pub const GET_CPU_VERSION : u32 = 1;
pub const EXTENDED_PROCESSOR_SIGNATURE_AND_FEATURE : u32 = 0x80000001;

bitflags! {
    pub struct CpuVersion : u32
    {
        const ECX_SSE3 = 0x1;
        const ECX_PCLMULQDQ = 0x2;
        const ECX_DTES64 = 0x4;
        const ECX_MONITOR = 0x8;
        const ECX_DS_CPL = 0x10;
        const ECX_VMX = 0x20;
        const ECX_SMX = 0x40;
        const ECX_EIST = 0x80;
        const ECX_TM2 = 0x100;
        const ECX_SSS3 = 0x200;
        const ECX_CNXT_ID = 0x400;
        const ECX_SDBG = 0x800;
        const ECX_FMA = 0x1000;
        const ECX_CMPXCHG16B = 0x2000;
        const ECX_XTPR = 0x4000;
        const ECX_PDCM = 0x8000;
        const ECX_PCID = 0x20000;
        const ECX_DCA = 0x40000;
        const ECX_SSE4_1 = 0x80000;
        const ECX_SSE4_2 = 0x100000;
        const ECX_X2APIC = 0x200000;
        const ECX_MOVBE = 0x400000;
        const ECX_POPCNT = 0x800000;
        const ECX_TSCD = 0x1000000;
        const ECX_AESNI = 0x2000000;
        const ECX_XSAVE = 0x4000000;
        const ECX_OSXSAVE = 0x8000000;
        const ECX_AVX = 0x10000000;
        const ECX_F16C = 0x20000000;
        const ECX_RDRAND = 0x40000000;
            // EDX
        const EDX_FPU = 0x1;      // 0 x87 FPU on Chip
        const EDX_VME = 0x2;      // 1 Virtual-8086 Mode Enhancement
        const EDX_DE = 0x4;       // 2 Debugging Extensions
        const EDX_PSE = 0x8;      // 3 Page Size Extensions
        const EDX_TSC = 0x10;      // 4 Time Stamp Counter
        const EDX_MSR = 0x20;      // 5 RDMSR and WRMSR Support
        const EDX_PAE = 0x40;      // 6 Physical Address Extensions
        const EDX_MCE = 0x80;      // 7 Machine Check Exception
        const EDX_CX8 = 0x100;      // 8 CMPXCHG8B Inst.
        const EDX_APIC = 0x200;     // 9 APIC on Chip
        const EDX_SEP = 0x800;      // 11 SYSENTER and SYSEXIT
        const EDX_MTRR = 0x1000;     // 12 Memory Type Range Registers
        const EDX_PGE = 0x2000;      // 13 PTE Global Bit
        const EDX_MCA = 0x4000;      // 14 Machine Check Architecture
        const EDX_CMOV = 0x8000;     // 15 Conditional Move/Compare Instruction
        const EDX_PAT = 0x10000;      // 16 Page Attribute Table
        const EDX_PSE36 = 0x20000;    // 17 Page Size Extension
        const EDX_PSN = 0x40000;      // 18 Processor Serial Number
        const EDX_CLFSH = 0x80000;    // 19 CLFLUSH instruction
        const EDX_DS = 0x200000;       // 21 Debug Store
        const EDX_ACPI = 0x400000;     // 22 Thermal Monitor and Clock Ctrl
        const EDX_MMX = 0x800000;      // 23 MMX Technology
        const EDX_FXSR = 0x1000000;     // 24 FXSAVE/FXRSTOR
        const EDX_SSE = 0x2000000;      // 25 SSE Extensions
        const EDX_SSE2 = 0x4000000;     // 26 SSE2 Extensions
        const EDX_SS = 0x8000000;       // 27 Self Snoop
        const EDX_HTT = 0x10000000;      // 28 Multi-threading
        const EDX_TM = 0x20000000;       // 29 Therm. Monitor
        const EDX_PBE = 0x80000000;      // 31 Pend. Brk. EN.
    }
}

bitflags!
{
    pub struct Cr0RegLabel : u64
    {
        const CR0_PE = 1 << 0; // Protection Enable 启用保护模式
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

#[inline(always)]
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

#[inline(always)]
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

#[inline(always)]
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

#[inline(always)]
pub unsafe fn flush_tlb(vaddr : *const c_void)
{
    asm!(
        "invlpg [{bad_page}]",
        bad_page = in(reg) vaddr
    )
}