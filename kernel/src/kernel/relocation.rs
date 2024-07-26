use proc_macro::__init;

use super::cpu::get_cpu_number;
use super::elf64::{Elf64Shdr, Elf64Phdr, Elf64Ehdr};
use super::io::{self, IdeCtrlT, IDE_IOBASE_PRIMARY, IDE_LBA_MASTER, IDE_FEATURE, IDE_SECTOR, IDE_LBA_LOW, IDE_LBA_MID, IDE_LBA_HIGH, IDE_HDDEVSEL, outb, IDE_SR_BSY, IDE_SR_ERR, IDE_ALT_STATUS, IDE_DATA, inw, SECTOR_SIZE, inb, IDE_SR_DRDY, IDE_CMD_READ, IDE_COMMAND, IDE_SR_DRQ};
use super::sched;
use core::arch::asm;
use core::ptr::null_mut;
use super::string::memset;
use core::ffi::c_char;
use core::mem::size_of;
const SHF_ALLOC : u64 = 0b10;

pub static mut KERNEL_SIZE : usize = 0;


#[__init]
unsafe fn system_relocate64(elf64_shdr : *mut Elf64Shdr, base_addr : u64)
{
    let mut reloc_info;
    unsafe
    {
        reloc_info = ((*elf64_shdr).sh_addr) as *mut Elf64Rela;
        let rela_num = (*elf64_shdr).sh_size / size_of::<Elf64Rela>() as u64;
        let mut var = 0;
        while var < rela_num
        {
            match (*reloc_info).r_type & 0xffffffff {
                R_X86_64_RELATIVE => RX86_64Relative_Relocate(reloc_info, base_addr),
                _ => panic!("unknown relocation type")
            }
            reloc_info = reloc_info.offset(1);
            var += 1;
        }
    }
}

#[__init]
#[inline(always)]
unsafe fn RX86_64Relative_Relocate(elf64_rela : *mut Elf64Rela, base_addr : u64)
{
    *((*elf64_rela).r_offset as *mut u64) += (*elf64_rela).r_addend | 0xffff8 << 44;
}


const R_X86_64_NONE : u32 = 0;
const R_X86_64_64 : u32 = 1;
const R_X86_64_PC32 : u32 = 2;
const R_X86_64_GOT32 : u32 = 3;
const R_X86_64_PLT32 : u32 = 4;
const R_X86_64_COPY : u32 = 5;
const R_X86_64_GLOB_DAT : u32 = 6;
const R_X86_64_JUMP_SLOT : u32 = 7;
const R_X86_64_RELATIVE : u32 = 8;
const R_X86_64_GOTPCREL : u32 = 9;
const R_X86_64_32 : u32 = 10;
const R_X86_64_32S : u32 = 11;
const R_X86_64_16 : u32 = 12;
const R_X86_64_PC16 : u32 = 13;
const R_X86_64_8 : u32 = 14;
const R_X86_64_PC8 : u32 = 15;
const R_X86_64_DTPMOD64 : u32 = 16;
const R_X86_64_DTPOFF64 : u32 = 17;
const R_X86_64_TPOFF64 : u32 = 18;
const R_X86_64_TLSGD : u32 = 19;
const R_X86_64_TLSLD : u32 = 20;
const R_X86_64_DTPOFF32 : u32 = 21;
const R_X86_64_GOTTPOFF : u32 = 22;
const R_X86_64_TPOFF32 : u32 = 23;
const R_X86_64_PC64 : u32 = 24;
const R_X86_64_GOTOFF64 : u32 = 25;
const R_X86_64_GOTPC32 : u32 = 26;
const R_X86_64_GOT64 : u32 = 27;
const R_X86_64_GOTPCREL64 : u32 = 28;
const R_X86_64_GOTPC64 : u32 = 29;
const R_X86_64_GOTPLT64 : u32 = 30;
const R_X86_64_PLTOFF64 : u32 = 31;
const R_X86_64_SIZE32 : u32 = 32;
const R_X86_64_SIZE64 : u32 = 33;
const R_X86_64_GOTPC32_TLSDESC : u32 = 34;
const R_X86_64_TLSDESC_CALL : u32 = 35;
const R_X86_64_TLSDESC : u32 = 36;
const R_X86_64_IRELATIVE : u32 = 37;
const R_X86_64_GOTPCRELX : u32 = 41;
const R_X86_64_REX_GOTPCRELX : u32 = 42;

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
#[repr(packed)]
struct Elf64Rela
{
    r_offset : u64,               /* Address */
    r_type : u32,                 /* Relocation type and symbol index */
    r_sym : u32,                 
    r_addend : u64                /* Addend */
}

#[__init]
fn ide_early_select_sector(iobase : u16, selector : u8, lba : u64, cnt : u8)
{
    unsafe
    {
        outb(iobase + IDE_FEATURE, 0);
        outb(iobase + IDE_SECTOR, cnt);
        outb(iobase + IDE_LBA_LOW, (lba & 0xff) as u8);
        outb(iobase + IDE_LBA_MID, (lba >> 8 & 0xff) as u8);
        outb(iobase + IDE_LBA_HIGH, (lba >> 16 & 0xff) as u8);
        outb(iobase + IDE_HDDEVSEL, (lba >> 24 & 0xf) as u8 | selector);
    }
}



#[__init]
fn ide_early_pio_read_sector(iobase : u16, mut offset : *mut u16)
{
    let mut cnt = 0;
    unsafe
    {
        while cnt < SECTOR_SIZE / 2
        {
            *offset = inw(iobase + IDE_DATA);
            offset = offset.offset(1);
            cnt += 1;
        }
    }
}

#[__init]
#[inline(always)]
fn ide_early_busy_wait(io_base : u16 ,mask : u8)
{
    loop {
        let state = inb(io_base + IDE_ALT_STATUS);
        if state & IDE_SR_ERR != 0
        {
            panic!()
        }
        if state & IDE_SR_BSY != 0
        {
            continue;
        }
        if (state & mask) == mask
        {
            break;
        }
    }

}

#[__init]
fn ide_early_pio_sync_read(start_block : u32, num_blocks : u8, dst : *mut u8)
{
    let blocks = if num_blocks == 0
    {
        256
    }
    else
    {
        num_blocks as u64
    };
    let mut var = 0u64;
    io::outb(IDE_IOBASE_PRIMARY + IDE_HDDEVSEL, IDE_LBA_MASTER);
    ide_early_busy_wait(IDE_IOBASE_PRIMARY, IDE_SR_DRDY);
    ide_early_select_sector(IDE_IOBASE_PRIMARY, IDE_LBA_MASTER, start_block as u64, num_blocks);
    outb(IDE_IOBASE_PRIMARY + IDE_COMMAND, IDE_CMD_READ);
    while var < blocks {
        ide_early_busy_wait(IDE_IOBASE_PRIMARY, IDE_SR_DRQ);
        ide_early_pio_read_sector(IDE_IOBASE_PRIMARY, (dst as u64 + SECTOR_SIZE * var) as *mut u16);
        var += 1;
    }
}

#[__init]
unsafe fn load_system_section(elf64_phdr : *mut Elf64Phdr, kernel_size : &mut usize)
{
    let mut blocks;
    let mut block_num = 0u64;
    if (*elf64_phdr).p_flags | (SegmentType::PtLoad as u32) > 0
    {
        // Calculate Load Infomation
        blocks = (*elf64_phdr).p_filesz.div_ceil(io::SECTOR_SIZE);
        if blocks == 0
        {
            load_bss(elf64_phdr);
        }
        else
        {
            while blocks > 255
            {
                ide_early_pio_sync_read((((*elf64_phdr).p_offset / io::SECTOR_SIZE) + 10 + block_num * 256) as u32, 0, ((*elf64_phdr).p_paddr + 256 * io::SECTOR_SIZE * block_num) as *mut u8);
                block_num += 1;
                blocks -= 256;
            }
            ide_early_pio_sync_read((((*elf64_phdr).p_offset / io::SECTOR_SIZE) + 10 + block_num * 256) as u32, (blocks + ((*elf64_phdr).p_filesz % io::SECTOR_SIZE != 0) as u64) as u8, ((*elf64_phdr).p_paddr + 256 * io::SECTOR_SIZE * block_num - (*elf64_phdr).p_paddr % io::SECTOR_SIZE) as *mut u8);
        }
        if ((*elf64_phdr).p_paddr + (*elf64_phdr).p_filesz) as usize > KERNEL_SIZE
        {
            *kernel_size = ((*elf64_phdr).p_paddr + (*elf64_phdr).p_memsz) as usize;
        }
    }
}

#[__init]
unsafe fn load_bss(elf64_phdr : *mut Elf64Phdr)
{
    // memset((*elf64_phdr).p_paddr as *mut u8, 0, (*elf64_phdr).p_memsz as usize);
    let mut start_ptr = (*elf64_phdr).p_paddr as *mut u64;
    let mut var = (*elf64_phdr).p_memsz as isize;
    while var > 0 {
        *start_ptr = 0;
        var -= 8;
        start_ptr = start_ptr.offset(1);
    }
}


#[__init]
pub unsafe fn process_relocation(elf64_ehdr : *mut Elf64Ehdr, base_addr : u64)
{
    let mut shdr;
    let mut var = 0;
    shdr = (*elf64_ehdr).e_shoff as *mut Elf64Shdr;
    while var < (*elf64_ehdr).e_shnum
    {
        if (*shdr).sh_type == 4
        {
            system_relocate64(shdr, base_addr);
        }
        shdr = shdr.offset(1);
        var += 1;
    }
}

#[__init]
#[no_mangle]
pub unsafe fn kernel_relocation(elf64_ehdr : *mut Elf64Ehdr)
{
    let mut shdr;
    let mut phdr;
    let mut var = 1;
    let mut kernel_size = 0;
    unsafe {
        phdr = ((elf64_ehdr as u64 + (*elf64_ehdr).e_phoff) as *mut Elf64Phdr).offset(1);
        
        while var < (*elf64_ehdr).e_phnum
        {
            if (*phdr).p_type == (SegmentType::PtLoad as u32)
            {
                load_system_section(phdr, &mut kernel_size);
            }
            phdr = phdr.offset(1);
            var += 1;
        }
        let start_pos = (*elf64_ehdr).e_shoff - (*elf64_ehdr).e_shoff % io::SECTOR_SIZE;
        ide_early_pio_sync_read(((start_pos / io::SECTOR_SIZE) + 10) as u32, ((*elf64_ehdr).e_shoff + ((*elf64_ehdr).e_shnum  as u64) * (size_of::<Elf64Shdr>() as u64) - start_pos).div_ceil(io::SECTOR_SIZE) as u8 , (elf64_ehdr as *mut u8).offset(4096));
        shdr = ((elf64_ehdr as u64 + (*elf64_ehdr).e_shoff % 512) + 4096) as *mut Elf64Shdr;
        var = 0;
        while var < (*elf64_ehdr).e_shnum
        {
            if (*shdr).sh_type == 4
            {
                system_relocate64(shdr, 0);
            }
            shdr = shdr.offset(1);
            var += 1;
        }
        KERNEL_SIZE = kernel_size;
    }
}