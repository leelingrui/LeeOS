use super::io::{self, IdeCtrlT};
use super::string::memset;
use core::ffi::c_char;
use core::mem::size_of;
const SHF_ALLOC : u64 = 0b10;
const EI_NIDENT : usize = 0x10;

#[repr(align(1))]
#[repr(C)]
pub struct Elf64Ehdr
{
    e_ident : [c_char ; EI_NIDENT],
    e_type : u16,
    e_machine : u16,
    e_version : u32,
    e_entry : u64,
    e_phoff : u64,
    e_shoff : u64,
    e_flags : u32,
    e_ehsize : u16,
    e_phentsize : u16,
    e_phnum : u16,
    e_shentsize : u16,
    e_shnum : u16,
    e_shstrndx : u16
}

#[repr(align(1))]
#[repr(C)]
struct Elf64Phdr
{
    p_type : u32,			/* Segment type */
    p_flags : u32,		/* Segment flags */
    p_offset : u64,		/* Segment file offset */
    p_vaddr : u64,		/* Segment virtual address */
    p_paddr : u64,		/* Segment physical address */
    p_filesz : u64,		/* Segment size in file */
    p_memsz : u64,		/* Segment size in memory */
    p_align : u64		/* Segment alignment */
}
#[repr(align(1))]
#[repr(C)]
struct Elf64Shdr
{
    sh_name : u32,		/* Section name (string tbl index) */
    sh_type : u32,		/* Section type */
    sh_flags : u64,		/* Section flags */
    sh_addr : u64,		/* Section virtual addr at execution */
    sh_offset : u64,		/* Section file offset */
    sh_size : u64,		/* Section size in bytes */
    sh_link : u32,		/* Link to another section */
    sh_info : u32,		/* Additional section information */
    sh_addralign : u64,		/* Section alignment */
    sh_entsize : u64		/* Entry size if section holds table */
}
#[repr(C)]
#[repr(align(1))]
struct Elf64Rela
{
    r_offset : u64,               /* Address */
    r_info : u64,                 /* Relocation type and symbol index */
    r_addend : u64               /* Addend */
}



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
                _ => ()
            }
            reloc_info = reloc_info.offset(1);
            var += 1;
        }
    }
}

unsafe fn RX86_64Relative_Relocate(elf64_rela : *mut Elf64Rela)
{
    *((*elf64_rela).r_offset as *mut u64) += (*elf64_rela).r_addend;
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

unsafe fn load_system_section(disk : &io::IdeDiskT, elf64_phdr : *mut Elf64Phdr)
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
                io::ide_pio_sync_read(&disk, (((*elf64_phdr).p_offset / io::SECTOR_SIZE) + 10) as u32, 255, ((*elf64_phdr).p_paddr + 256 * io::SECTOR_SIZE * block_num) as *mut u8);
                block_num += 1;
                block_num -= 256;
            }
            io::ide_pio_sync_read(&disk,(((*elf64_phdr).p_offset / io::SECTOR_SIZE) + 10) as u32, (blocks + ((*elf64_phdr).p_filesz % io::SECTOR_SIZE != 0) as u64) as u8, ((*elf64_phdr).p_paddr + 256 * io::SECTOR_SIZE * block_num - (*elf64_phdr).p_paddr % io::SECTOR_SIZE) as *mut u8);
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
    let mut _ctrl = io::IdeCtrlT::new(io::IDE_IOBASE_PRIMARY);
    let _disk = io::IdeDiskT::new(&mut _ctrl as *mut IdeCtrlT, io::IDE_LBA_MASTER, true, 0xfffffff);
    unsafe {
        phdr = ((elf64_ehdr as u64 + (*elf64_ehdr).e_phoff) as *mut Elf64Phdr).offset(1);
        
        while var < (*elf64_ehdr).e_phnum
        {
            if (*phdr).p_type == (SegmentType::PtLoad as u32)
            {
                load_system_section(&_disk, phdr);
            }
            phdr = phdr.offset(1);
            var += 1;
        }
        let start_pos = (*elf64_ehdr).e_shoff - (*elf64_ehdr).e_shoff % io::SECTOR_SIZE;
        io::ide_pio_sync_read(&_disk, ((start_pos / io::SECTOR_SIZE) + 10) as u32, ((*elf64_ehdr).e_shoff + ((*elf64_ehdr).e_shnum  as u64) * (size_of::<Elf64Shdr>() as u64) - start_pos).div_ceil(io::SECTOR_SIZE) as u8 , (elf64_ehdr as *mut u8).offset(4096));
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