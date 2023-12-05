use super::cpu::get_cpu_number;
use super::elf64::{Elf64Shdr, Elf64Phdr, Elf64Ehdr};
use super::io::{self, IdeCtrlT};
use super::sched;
use core::arch::asm;
use core::ptr::null_mut;
use super::string::memset;
use core::ffi::c_char;
use core::mem::size_of;
const SHF_ALLOC : u64 = 0b10;

pub static mut KERNEL_SIZE : usize = 0;



unsafe fn system_relocate64(elf64_shdr : *mut Elf64Shdr)
{
    let mut reloc_info;
    unsafe
    {
        reloc_info = ((*elf64_shdr).sh_addr) as *mut Elf64Rela;
        let rela_num = (*elf64_shdr).sh_size / size_of::<Elf64Rela>() as u64;
        let mut var = 0;
        while var < rela_num
        {
            match (*reloc_info).r_info {
                R_X86_64_RELATIVE => RX86_64Relative_Relocate(reloc_info),
                _ => panic!("unknown relocation type")
            }
            reloc_info = reloc_info.offset(1);
            var += 1;
        }
    }
}

unsafe fn RX86_64Relative_Relocate(elf64_rela : *mut Elf64Rela)
{
    *((*elf64_rela).r_offset as *mut u64) += (*elf64_rela).r_addend | 0xffff8 << 44;
}


const R_X86_64_NONE : u64 = 0;
const R_X86_64_64 : u64 = 1;
const R_X86_64_PC32 : u64 = 2;
const R_X86_64_GOT32 : u64 = 3;
const R_X86_64_PLT32 : u64 = 4;
const R_X86_64_COPY : u64 = 5;
const R_X86_64_GLOB_DAT : u64 = 6;
const R_X86_64_JUMP_SLOT : u64 = 7;
const R_X86_64_RELATIVE : u64 = 8;
const R_X86_64_GOTPCREL : u64 = 9;
const R_X86_64_32 : u64 = 10;
const R_X86_64_32S : u64 = 11;
const R_X86_64_16 : u64 = 12;
const R_X86_64_PC16 : u64 = 13;
const R_X86_64_8 : u64 = 14;
const R_X86_64_PC8 : u64 = 15;
const R_X86_64_DTPMOD64 : u64 = 16;
const R_X86_64_DTPOFF64 : u64 = 17;
const R_X86_64_TPOFF64 : u64 = 18;
const R_X86_64_TLSGD : u64 = 19;
const R_X86_64_TLSLD : u64 = 20;
const R_X86_64_DTPOFF32 : u64 = 21;
const R_X86_64_GOTTPOFF : u64 = 22;
const R_X86_64_TPOFF32 : u64 = 23;
const R_X86_64_PC64 : u64 = 24;
const R_X86_64_GOTOFF64 : u64 = 25;
const R_X86_64_GOTPC32 : u64 = 26;
const R_X86_64_GOT64 : u64 = 27;
const R_X86_64_GOTPCREL64 : u64 = 28;
const R_X86_64_GOTPC64 : u64 = 29;
const R_X86_64_GOTPLT64 : u64 = 30;
const R_X86_64_PLTOFF64 : u64 = 31;
const R_X86_64_SIZE32 : u64 = 32;
const R_X86_64_SIZE64 : u64 = 33;
const R_X86_64_GOTPC32_TLSDESC : u64 = 34;
const R_X86_64_TLSDESC_CALL : u64 = 35;
const R_X86_64_TLSDESC : u64 = 36;
const R_X86_64_IRELATIVE : u64 = 37;
const R_X86_64_GOTPCRELX : u64 = 41;
const R_X86_64_REX_GOTPCRELX : u64 = 42;

enum SegmentType
{
    PtNull = 0,    // 未使用
    PtLoad = 1,    // 可加载程序段
    PtDynamic = 2, // 动态加载信息
    PtInterp = 3,  // 动态加载器名称
    PtNote = 4,    // 一些辅助信息
    PtShlib = 5,   // 保留
    PtPhdr = 6,    // 程序头表
    PtLoproc = 0x70000000,
    PtHiproc = 0x7fffffff,
}

#[repr(C)]
#[repr(align(1))]
struct Elf64Rela
{
    r_offset : u64,               /* Address */
    r_info : u64,                 /* Relocation type and symbol index */
    r_addend : u64               /* Addend */
}

unsafe fn load_system_section(elf64_phdr : *mut Elf64Phdr)
{
    let blocks;
    let mut block_num = 0u64;
    if (*elf64_phdr).p_flags | (SegmentType::PtLoad as u32) > 0
    {
        // Calculate Load Infomation
        blocks = (*elf64_phdr).p_filesz / io::SECTOR_SIZE;
        if blocks == 0
        {
            load_bss(elf64_phdr);
        }
        else
        {
            while blocks > 255
            {
                io::ide_early_pio_sync_read((((*elf64_phdr).p_offset / io::SECTOR_SIZE) + 10) as u32, 0, ((*elf64_phdr).p_paddr + 256 * io::SECTOR_SIZE * block_num) as *mut u8);
                block_num += 1;
                block_num -= 256;
            }
            io::ide_early_pio_sync_read((((*elf64_phdr).p_offset / io::SECTOR_SIZE) + 10) as u32, (blocks + ((*elf64_phdr).p_filesz % io::SECTOR_SIZE != 0) as u64) as u8, ((*elf64_phdr).p_paddr + 256 * io::SECTOR_SIZE * block_num - (*elf64_phdr).p_paddr % io::SECTOR_SIZE) as *mut u8);
        }
        if ((*elf64_phdr).p_paddr + (*elf64_phdr).p_filesz) as usize > KERNEL_SIZE
        {
            KERNEL_SIZE = ((*elf64_phdr).p_paddr + (*elf64_phdr).p_memsz) as usize;
        }
    }
}

unsafe fn load_bss(elf64_phdr : *mut Elf64Phdr)
{
    memset((*elf64_phdr).p_paddr as *mut u8, 0, (*elf64_phdr).p_memsz as usize);
}


#[no_mangle]
pub unsafe fn kernel_relocation(elf64_ehdr : *mut Elf64Ehdr)
{
    let mut shdr;
    let mut phdr;
    let mut var = 1;
    unsafe {
        phdr = ((elf64_ehdr as u64 + (*elf64_ehdr).e_phoff) as *mut Elf64Phdr).offset(1);
        
        while var < (*elf64_ehdr).e_phnum
        {
            if (*phdr).p_type == (SegmentType::PtLoad as u32)
            {
                load_system_section(phdr);
            }
            phdr = phdr.offset(1);
            var += 1;
        }
        let start_pos = (*elf64_ehdr).e_shoff - (*elf64_ehdr).e_shoff % io::SECTOR_SIZE;
        io::ide_early_pio_sync_read(((start_pos / io::SECTOR_SIZE) + 10) as u32, ((*elf64_ehdr).e_shoff + ((*elf64_ehdr).e_shnum  as u64) * (size_of::<Elf64Shdr>() as u64) - start_pos).div_ceil(io::SECTOR_SIZE) as u8 , (elf64_ehdr as *mut u8).offset(4096));
        shdr = ((elf64_ehdr as u64 + (*elf64_ehdr).e_shoff % 512) + 4096) as *mut Elf64Shdr;
        var = 0;
        while var < (*elf64_ehdr).e_shnum
        {
            if (*shdr).sh_type == 4
            {
                system_relocate64(shdr);
            }
            shdr = shdr.offset(1);
            var += 1;
        }
    }
}