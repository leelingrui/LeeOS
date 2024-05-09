use core::{ffi::{c_char, c_void}, mem::size_of, ptr::null_mut, iter::empty};

use crate::{fs::file::{File, EOF, FS}, mm::{mmap::{sys_mmap, __do_mmap}, memory::PAGE_SIZE, mm_type::MmapType}};

use super::{Off, io};


const EI_NIDENT : usize = 0x10;

const EM_NONE : u16 = 0;           // No machine
const EM_M32 : u16 = 1;            // AT&T WE 32100
const EM_SPARC : u16 = 2;          // SPARC
const EM_386 : u16 = 3;            // Intel 386
const EM_68K : u16 = 4;            // Motorola 68000
const EM_88K : u16 = 5;            // Motorola 88000
const EM_IAMCU : u16 = 6;          // Intel MCU
const EM_860 : u16 = 7;            // Intel 80860
const EM_MIPS : u16 = 8;           // MIPS R3000
const EM_S370 : u16 = 9;           // IBM System/370
const EM_MIPS_RS3_LE : u16 = 10;   // MIPS RS3000 Little-endian
const EM_PARISC : u16 = 15;        // Hewlett-Packard PA-RISC
const EM_VPP500 : u16 = 17;        // Fujitsu VPP500
const EM_SPARC32PLUS : u16 = 18;   // Enhanced instruction set SPARC
const EM_960 : u16 = 19;           // Intel 80960
const EM_PPC : u16 = 20;           // PowerPC
const EM_PPC64 : u16 = 21;         // PowerPC64
const EM_S390 : u16 = 22;          // IBM System/390
const EM_SPU : u16 = 23;           // IBM SPU/SPC
const EM_V800 : u16 = 36;          // NEC V800
const EM_FR20 : u16 = 37;          // Fujitsu FR20
const EM_RH32 : u16 = 38;          // TRW RH-32
const EM_RCE : u16 = 39;           // Motorola RCE
const EM_ARM : u16 = 40;           // ARM
const EM_ALPHA : u16 = 41;         // DEC Alpha
const EM_SH : u16 = 42;            // Hitachi SH
const EM_SPARCV9 : u16 = 43;       // SPARC V9
const EM_TRICORE : u16 = 44;       // Siemens TriCore
const EM_ARC : u16 = 45;           // Argonaut RISC Core
const EM_H8_300 : u16 = 46;        // Hitachi H8/300
const EM_H8_300H : u16 = 47;       // Hitachi H8/300H
const EM_H8S : u16 = 48;           // Hitachi H8S
const EM_H8_500 : u16 = 49;        // Hitachi H8/500
const EM_IA_64 : u16 = 50;         // Intel IA-64 processor architecture
const EM_MIPS_X : u16 = 51;        // Stanford MIPS-X
const EM_COLDFIRE : u16 = 52;      // Motorola ColdFire
const EM_68HC12 : u16 = 53;        // Motorola M68HC12
const EM_MMA : u16 = 54;           // Fujitsu MMA Multimedia Accelerator
const EM_PCP : u16 = 55;           // Siemens PCP
const EM_NCPU : u16 = 56;          // Sony nCPU embedded RISC processor
const EM_NDR1 : u16 = 57;          // Denso NDR1 microprocessor
const EM_STARCORE : u16 = 58;      // Motorola Star*Core processor
const EM_ME16 : u16 = 59;          // Toyota ME16 processor
const EM_ST100 : u16 = 60;         // STMicroelectronics ST100 processor
const EM_TINYJ : u16 = 61;         // Advanced Logic Corp. TinyJ embedded processor family
const EM_X86_64 : u16 = 62;        // AMD x86-64 architecture
const EM_PDSP : u16 = 63;          // Sony DSP Processor
const EM_PDP10 : u16 = 64;         // Digital Equipment Corp. PDP-10
const EM_PDP11 : u16 = 65;         // Digital Equipment Corp. PDP-11
const EM_FX66 : u16 = 66;          // Siemens FX66 microcontroller
const EM_ST9PLUS : u16 = 67;       // STMicroelectronics ST9+ 8/16 bit microcontroller
const EM_ST7 : u16 = 68;           // STMicroelectronics ST7 8-bit microcontroller
const EM_68HC16 : u16 = 69;        // Motorola MC68HC16 Microcontroller
const EM_68HC11 : u16 = 70;        // Motorola MC68HC11 Microcontroller
const EM_68HC08 : u16 = 71;        // Motorola MC68HC08 Microcontroller
const EM_68HC05 : u16 = 72;        // Motorola MC68HC05 Microcontroller
const EM_SVX : u16 = 73;           // Silicon Graphics SVx
const EM_ST19 : u16 = 74;          // STMicroelectronics ST19 8-bit microcontroller
const EM_VAX : u16 = 75;           // Digital VAX
const EM_CRIS : u16 = 76;          // Axis Communications 32-bit embedded processor
const EM_JAVELIN : u16 = 77;       // Infineon Technologies 32-bit embedded processor
const EM_FIREPATH : u16 = 78;      // Element 14 64-bit DSP Processor
const EM_ZSP : u16 = 79;           // LSI Logic 16-bit DSP Processor
const EM_MMIX : u16 = 80;          // Donald Knuth's educational 64-bit processor
const EM_HUANY : u16 = 81;         // Harvard University machine-independent object files
const EM_PRISM : u16 = 82;         // SiTera Prism
const EM_AVR : u16 = 83;           // Atmel AVR 8-bit microcontroller
const EM_FR30 : u16 = 84;          // Fujitsu FR30
const EM_D10V : u16 = 85;          // Mitsubishi D10V
const EM_D30V : u16 = 86;          // Mitsubishi D30V
const EM_V850 : u16 = 87;          // NEC v850
const EM_M32R : u16 = 88;          // Mitsubishi M32R
const EM_MN10300 : u16 = 89;       // Matsushita MN10300
const EM_MN10200 : u16 = 90;       // Matsushita MN10200
const EM_PJ : u16 = 91;            // picoJava
const EM_OPENRISC : u16 = 92;      // OpenRISC 32-bit embedded processor
const EM_ARC_COMPACT : u16 = 93;   // ARC International ARCompact processor (old
                           // spelling/synonym: EM_ARC_A5)
const EM_XTENSA : u16 = 94;        // Tensilica Xtensa Architecture
const EM_VIDEOCORE : u16 = 95;     // Alphamosaic VideoCore processor
const EM_TMM_GPP : u16 = 96;       // Thompson Multimedia General Purpose Processor
const EM_NS32K : u16 = 97;         // National Semiconductor 32000 series
const EM_TPC : u16 = 98;           // Tenor Network TPC processor
const EM_SNP1K : u16 = 99;         // Trebia SNP 1000 processor
const EM_ST200 : u16 = 100;        // STMicroelectronics (www.st.com) ST200
const EM_IP2K : u16 = 101;         // Ubicom IP2xxx microcontroller family
const EM_MAX : u16 = 102;          // MAX Processor
const EM_CR : u16 = 103;           // National Semiconductor CompactRISC microprocessor
const EM_F2MC16 : u16 = 104;       // Fujitsu F2MC16
const EM_MSP430 : u16 = 105;       // Texas Instruments embedded microcontroller msp430
const EM_BLACKFIN : u16 = 106;     // Analog Devices Blackfin (DSP) processor
const EM_SE_C33 : u16 = 107;       // S1C33 Family of Seiko Epson processors
const EM_SEP : u16 = 108;          // Sharp embedded microprocessor
const EM_ARCA : u16 = 109;         // Arca RISC Microprocessor
const EM_UNICORE : u16 = 110;      // Microprocessor series from PKU-Unity Ltd. and MPRC
                           // of Peking University
const EM_EXCESS : u16 = 111;       // eXcess: 16/32/64-bit configurable embedded CPU
const EM_DXP : u16 = 112;          // Icera Semiconductor Inc. Deep Execution Processor
const EM_ALTERA_NIOS2 : u16 = 113; // Altera Nios II soft-core processor
const EM_CRX : u16 = 114;          // National Semiconductor CompactRISC CRX
const EM_XGATE : u16 = 115;        // Motorola XGATE embedded processor
const EM_C166 : u16 = 116;         // Infineon C16x/XC16x processor
const EM_M16C : u16 = 117;         // Renesas M16C series microprocessors
const EM_DSPIC30F : u16 = 118;     // Microchip Technology dsPIC30F Digital Signal
                           // Controller
const EM_CE : u16 = 119;           // Freescale Communication Engine RISC core
const EM_M32C : u16 = 120;         // Renesas M32C series microprocessors
const EM_TSK3000 : u16 = 131;      // Altium TSK3000 core
const EM_RS08 : u16 = 132;         // Freescale RS08 embedded processor
const EM_SHARC : u16 = 133;        // Analog Devices SHARC family of 32-bit DSP
                           // processors
const EM_ECOG2 : u16 = 134;        // Cyan Technology eCOG2 microprocessor
const EM_SCORE7 : u16 = 135;       // Sunplus S+core7 RISC processor
const EM_DSP24 : u16 = 136;        // New Japan Radio (NJR) 24-bit DSP Processor
const EM_VIDEOCORE3 : u16 = 137;   // Broadcom VideoCore III processor
const EM_LATTICEMICO32 : u16 = 138; // RISC processor for Lattice FPGA architecture
const EM_SE_C17 : u16 = 139;        // Seiko Epson C17 family
const EM_TI_C6000 : u16 = 140;      // The Texas Instruments TMS320C6000 DSP family
const EM_TI_C2000 : u16 = 141;      // The Texas Instruments TMS320C2000 DSP family
const EM_TI_C5500 : u16 = 142;      // The Texas Instruments TMS320C55x DSP family
const EM_MMDSP_PLUS : u16 = 160;    // STMicroelectronics 64bit VLIW Data Signal Processor
const EM_CYPRESS_M8C : u16 = 161;   // Cypress M8C microprocessor
const EM_R32C : u16 = 162;          // Renesas R32C series microprocessors
const EM_TRIMEDIA : u16 = 163;      // NXP Semiconductors TriMedia architecture family
const EM_HEXAGON : u16 = 164;       // Qualcomm Hexagon processor
const EM_8051 : u16 = 165;          // Intel 8051 and variants
const EM_STXP7X : u16 = 166;        // STMicroelectronics STxP7x family of configurable
                            // and extensible RISC processors
const EM_NDS32 : u16 = 167;         // Andes Technology compact code size embedded RISC
                            // processor family
const EM_ECOG1 : u16 = 168;         // Cyan Technology eCOG1X family
const EM_ECOG1X : u16 = 168;        // Cyan Technology eCOG1X family
const EM_MAXQ30 : u16 = 169;        // Dallas Semiconductor MAXQ30 Core Micro-controllers
const EM_XIMO16 : u16 = 170;        // New Japan Radio (NJR) 16-bit DSP Processor
const EM_MANIK : u16 = 171;         // M2000 Reconfigurable RISC Microprocessor
const EM_CRAYNV2 : u16 = 172;       // Cray Inc. NV2 vector architecture
const EM_RX : u16 = 173;            // Renesas RX family
const EM_METAG : u16 = 174;         // Imagination Technologies META processor
                            // architecture
const EM_MCST_ELBRUS : u16 = 175;   // MCST Elbrus general purpose hardware architecture
const EM_ECOG16 : u16 = 176;        // Cyan Technology eCOG16 family
const EM_CR16 : u16 = 177;          // National Semiconductor CompactRISC CR16 16-bit
                            // microprocessor
const EM_ETPU : u16 = 178;          // Freescale Extended Time Processing Unit
const EM_SLE9X : u16 = 179;         // Infineon Technologies SLE9X core
const EM_L10M : u16 = 180;          // Intel L10M
const EM_K10M : u16 = 181;          // Intel K10M
const EM_AARCH64 : u16 = 183;       // ARM AArch64
const EM_AVR32 : u16 = 185;         // Atmel Corporation 32-bit microprocessor family
const EM_STM8 : u16 = 186;          // STMicroeletronics STM8 8-bit microcontroller
const EM_TILE64 : u16 = 187;        // Tilera TILE64 multicore architecture family
const EM_TILEPRO : u16 = 188;       // Tilera TILEPro multicore architecture family
const EM_MICROBLAZE : u16 = 189;    // Xilinx MicroBlaze 32-bit RISC soft processor core
const EM_CUDA : u16 = 190;          // NVIDIA CUDA architecture
const EM_TILEGX : u16 = 191;        // Tilera TILE-Gx multicore architecture family
const EM_CLOUDSHIELD : u16 = 192;   // CloudShield architecture family
const EM_COREA_1ST : u16 = 193;     // KIPO-KAIST Core-A 1st generation processor family
const EM_COREA_2ND : u16 = 194;     // KIPO-KAIST Core-A 2nd generation processor family
const EM_ARC_COMPACT2 : u16 = 195;  // Synopsys ARCompact V2
const EM_OPEN8 : u16 = 196;         // Open8 8-bit RISC soft processor core
const EM_RL78 : u16 = 197;          // Renesas RL78 family
const EM_VIDEOCORE5 : u16 = 198;    // Broadcom VideoCore V processor
const EM_78KOR : u16 = 199;         // Renesas 78KOR family
const EM_56800EX : u16 = 200;       // Freescale 56800EX Digital Signal Controller (DSC)
const EM_BA1 : u16 = 201;           // Beyond BA1 CPU architecture
const EM_BA2 : u16 = 202;           // Beyond BA2 CPU architecture
const EM_XCORE : u16 = 203;         // XMOS xCORE processor family
const EM_MCHP_PIC : u16 = 204;      // Microchip 8-bit PIC(r) family
const EM_INTEL205 : u16 = 205;      // Reserved by Intel
const EM_INTEL206 : u16 = 206;      // Reserved by Intel
const EM_INTEL207 : u16 = 207;      // Reserved by Intel
const EM_INTEL208 : u16 = 208;      // Reserved by Intel
const EM_INTEL209 : u16 = 209;      // Reserved by Intel
const EM_KM32 : u16 = 210;          // KM211 KM32 32-bit processor
const EM_KMX32 : u16 = 211;         // KM211 KMX32 32-bit processor
const EM_KMX16 : u16 = 212;         // KM211 KMX16 16-bit processor
const EM_KMX8 : u16 = 213;          // KM211 KMX8 8-bit processor
const EM_KVARC : u16 = 214;         // KM211 KVARC processor
const EM_CDP : u16 = 215;           // Paneve CDP architecture family
const EM_COGE : u16 = 216;          // Cognitive Smart Memory Processor
const EM_COOL : u16 = 217;          // iCelero CoolEngine
const EM_NORC : u16 = 218;          // Nanoradio Optimized RISC
const EM_CSR_KALIMBA : u16 = 219;   // CSR Kalimba architecture family
const EM_AMDGPU : u16 = 224;        // AMD GPU architecture
const EM_RISCV : u16 = 243;         // RISC-V
const EM_LANAI : u16 = 244;         // Lanai 32-bit processor
const EM_BPF : u16 = 247;           // Linux kernel bpf virtual machine
const EM_VE : u16 = 251;            // NEC SX-Aurora VE
const EM_CSKY : u16 = 252;          // C-SKY 32-bit processor
const EM_LOONGARCH : u16  = 258;     // LoongArch

const PF_X : u32 = 0x1; // 可执行
const PF_W : u32 = 0x2; // 可写
const PF_R : u32 = 0x4; // 可读

const PT_NULL : u32 = 0;    // 未使用
const PT_LOAD : u32 = 1;    // 可加载程序段
const PT_DYNAMIC : u32 = 2; // 动态加载信息
const PT_INTERP : u32 = 3;  // 动态加载器名称
const PT_NOTE : u32 = 4;    // 一些辅助信息
const PT_SHLIB : u32 = 5;   // 保留
const PT_PHDR : u32 = 6;    // 程序头表
const PT_LOPROC : u32 = 0x70000000;
const PT_HIPROC : u32 = 0x7fffffff;

#[repr(packed)]
#[repr(C)]
pub struct Elf64Ehdr
{
    pub e_ident : [c_char ; EI_NIDENT],
    pub e_type : u16,
    pub e_machine : u16,
    pub e_version : u32,
    pub e_entry : u64,
    pub e_phoff : u64,
    pub e_shoff : u64,
    pub e_flags : u32,
    pub e_ehsize : u16,
    pub e_phentsize : u16,
    pub e_phnum : u16,
    pub e_shentsize : u16,
    pub e_shnum : u16,
    pub e_shstrndx : u16
}

#[repr(align(1))]
#[repr(C)]
pub struct Elf64Phdr
{
    pub p_type : u32,			/* Segment type */
    pub p_flags : u32,		/* Segment flags */
    pub p_offset : u64,		/* Segment file offset */
    pub p_vaddr : u64,		/* Segment virtual address */
    pub p_paddr : u64,		/* Segment physical address */
    pub p_filesz : u64,		/* Segment size in file */
    pub p_memsz : u64,		/* Segment size in memory */
    pub p_align : u64		/* Segment alignment */
}
#[repr(align(1))]
#[repr(C)]
pub struct Elf64Shdr
{
    pub sh_name : u32,		/* Section name (string tbl index) */
    pub sh_type : u32,		/* Section type */
    pub sh_flags : u64,		/* Section flags */
    pub sh_addr : u64,		/* Section virtual addr at execution */
    pub sh_offset : u64,		/* Section file offset */
    pub sh_size : u64,		/* Section size in bytes */
    pub sh_link : u32,		/* Link to another section */
    pub sh_info : u32,		/* Additional section information */
    pub sh_addralign : u64,		/* Section alignment */
    pub sh_entsize : u64		/* Entry size if section holds table */
}

enum Etype
{
    EtNone = 0,        // 无类型
    EtRel = 1,         // 可重定位文件
    EtExec = 2,        // 可执行文件
    EtDyn = 3,         // 动态链接库
    EtCore = 4,        // core 文件，未说明，占位
    EtLoproc = 0xff00, // 处理器相关低值
    EtHiproc = 0xffff, // 处理器相关高值
}

enum EVersion
{
    EvNone = 0,    // 不可用版本
    EvCurrent = 1, // 当前版本
}

fn elf64_validate(ehdr : *const Elf64Ehdr) -> bool
{
    unsafe
    {
        // 不是 ELF 文件
        if (compiler_builtins::mem::memcmp(&(*ehdr).e_ident as *const i8 as *const u8, [0x7f, 0x45, 0x4c, 0x46, 02, 01, 01, 00, 00, 00, 00, 00, 00, 00, 00, 00].as_ptr(), 7)) != 0
        {
            return false;
        }
        // 不是可执行文件
        if !((*ehdr).e_type == Etype::EtExec as u16 || (*ehdr).e_type == Etype::EtDyn as u16)
        {
            return false;
        }
        // 不是 386 程序
        if (*ehdr).e_machine != EM_386
        {
            return false;
        }
        // 版本不可识别
        if (*ehdr).e_version != EVersion::EvCurrent as u32
        {
            return false;
        }
        if (*ehdr).e_phentsize != size_of::<Elf64Phdr>() as u16
        {
            return false;
        }
        return true;
    }
}

pub fn load_elf64(file_t : *mut File) -> i64
{
    unsafe
    {
        let ehdr = null_mut() as *mut Elf64Ehdr;
        FS.read_file(file_t, null_mut(), size_of::<Elf64Ehdr>(), 0);
        if elf64_validate(ehdr)
        {
            return EOF;
        }
        let mut phdr = (*ehdr).e_phoff as *mut Elf64Phdr;
        // __do_mmap(null_mut(), (*ehdr).e_phnum as usize * (*ehdr).e_phentsize as usize + size_of::<Elf64Ehdr>(), MmapType::PROT_KERNEL | MmapType::PROT_READ, MmapType::MAP_PRIVATE, null_mut(), 0);
        FS.read_file(file_t, (*ehdr).e_phoff as *mut c_void, (*ehdr).e_phnum as usize * (*ehdr).e_phentsize as usize, (*ehdr).e_phoff as Off);
        let mut var = 0;
        while var < (*ehdr).e_phnum {
            match (*phdr).p_type {
                PT_LOAD =>
                {
                    if !load_segment64(phdr, file_t)
                    {
                        return EOF;
                    }
                },
                _ => { }
            }
            phdr = phdr.offset(1);
            var += 1;
        }
        (*ehdr).e_entry as i64
    }
}

fn load_segment64(elf64_phdr : *mut Elf64Phdr, file_t : *mut File) -> bool
{
    unsafe
    {
        let mut prot = MmapType::empty();
        let mut flags = MmapType::empty();
        let loffset = (*elf64_phdr).p_vaddr as usize % PAGE_SIZE;
        let map_size = ((*elf64_phdr).p_memsz as usize + loffset).div_ceil(PAGE_SIZE) * PAGE_SIZE;
        // Calculate Load Infomation
        flags.insert(MmapType::MAP_PRIVATE);

        if (*elf64_phdr).p_flags & PF_W != 0
        {
            prot.insert(MmapType::PROT_WRITE);
        }

        if (*elf64_phdr).p_flags & PF_X != 0
        {
            prot.insert(MmapType::PROT_EXEC);
        }

        if (*elf64_phdr).p_flags & PF_R != 0
        {
            prot.insert(MmapType::PROT_READ);
        }
        let vma;
        if (*elf64_phdr).p_vaddr == 0
        {
            FS.read_file(file_t, null_mut(), (*elf64_phdr).p_filesz as usize, (*elf64_phdr).p_offset as usize);
            return true;
        }
        if (*elf64_phdr).p_filesz == 0
        {
            flags.insert(MmapType::MAP_ANONYMOUS);
            vma = __do_mmap(((*elf64_phdr).p_vaddr as usize - loffset) as *mut c_void, map_size, prot, flags, null_mut(), (*elf64_phdr).p_offset as usize - loffset);
        }
        else
        {
            vma = __do_mmap(((*elf64_phdr).p_vaddr as usize - loffset) as *mut c_void, map_size, prot, flags, file_t, (*elf64_phdr).p_offset as usize - loffset);

        }
        if vma.is_null()
        {
            false
        }
        else
        {
            true
        }
    }
}