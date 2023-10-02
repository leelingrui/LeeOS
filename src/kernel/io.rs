use core::alloc::GlobalAlloc;
use core::alloc::Layout;
use core::arch::asm;
use core::default;
use core::ffi::CStr;
use core::ffi::c_char;
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr::null;
use core::ptr::null_mut;

use alloc::string::String;

use crate::fs::PART_FS_EXTENDED;
use crate::fs::ext4::PartEntry;
use crate::fs::ext4::Idx;
use crate::fs::file::EOF;
use crate::kernel::interrupt::get_interrupt_state;
use crate::logk;
use crate::mm;
use crate::mm::memory;
use crate::mm::memory::MEMORY_POOL;
use crate::mm::memory::PAGE_SIZE;
use crate::printk;
use super::interrupt;
use super::semaphore;
use super::io;
use super::semaphore::SpinLock;

type LockT = bool;

pub const SECTOR_SIZE : u64 = 512;
pub const IDE_IOBASE_PRIMARY : u16 = 0x1f0;
const IDE_IOBASE_SECONDARY : u16 = 0x170;

const IDE_DATA : u16 = 0x0000;
const IDE_ERR : u16 = 0x0001;
const IDE_FEATURE : u16 = 0x0001;
const IDE_SECTOR : u16 = 0x0002;
const IDE_LBA_LOW : u16 = 0x0003;
const IDE_LBA_MID : u16 = 0x0004;
const IDE_LBA_HIGH : u16 = 0x0005;
const IDE_HDDEVSEL : u16 = 0x0006;
const IDE_STATUS : u16 = 0x0007;
const IDE_COMMAND : u16 = 0x0007;
const IDE_ALT_STATUS : u16 = 0x0206;
const IDE_CONTROL : u16 = 0x0206;
const IDE_DEVCTRL : u16 = 0x0206;

const IDE_CMD_READ : u8 = 0x20;
const IDE_CMD_WRITE : u8 = 0x30;
const IDE_CMD_IDENTIFY : u8 = 0xec;

// IDE 控制器状态寄存器
const IDE_SR_NULL : u8 = 0x00; // NULL
const IDE_SR_ERR : u8 = 0x01;  // Error
const IDE_SR_IDX : u8 = 0x02;  // Index
const IDE_SR_CORR : u8 = 0x04; // Corrected data
const IDE_SR_DRQ : u8 = 0x08;  // Data request
const IDE_SR_DSC : u8 = 0x10;  // Drive seek complete
const IDE_SR_DWF : u8 = 0x20;  // Drive write fault
const IDE_SR_DRDY : u8 = 0x40; // Drive ready
const IDE_SR_BSY : u8 = 0x80;  // Controller busy

// IDE 控制寄存器
const IDE_CTRL_HD15 : u8 = 0x00; // Use 4 bits for head (not used, was 0x08)
const IDE_CTRL_SRST : u8 = 0x04; // Soft reset
const IDE_CTRL_NIEN : u8 = 0x02; // Disable interrupts

// IDE 错误寄存器
const IDE_ER_AMNF : u8 = 0x01;  // Address mark not found
const IDE_ER_TK0NF : u8 = 0x02; // Track 0 not found
const IDE_ER_ABRT : u8 = 0x04;  // Abort
const IDE_ER_MCR : u8 = 0x08;   // Media change requested
const IDE_ER_IDNF : u8 = 0x10;  // Sector id not found
const IDE_ER_MC : u8 = 0x20;    // Media change
const IDE_ER_UNC : u8 = 0x40;   // Uncorrectable data error
const IDE_ER_BBK : u8 = 0x80;   // Bad block

pub const IDE_LBA_MASTER : u8 = 0b11100000; // 主盘 LBA
pub const IDE_LBA_SLAVE : u8 = 0b11110000;  // 从盘 LBA


pub static mut CONTROLLERS : [IdeCtrlT; IDE_CTRL_NR] = [IdeCtrlT::new(), IdeCtrlT::new()];


const IDE_CTRL_NR : usize = 2;
const IDE_DISK_NR : usize = 2;

#[repr(C)]
struct BootSector
{
    code : [u8; 446],
    entry : [PartEntry; 4],
    signature : u16
}

#[repr(C, packed)]
struct IdeParamsT
{
    config : u16,                 // 0 General configuration bits
    cylinders : u16,              // 01 cylinders
    reserved1 : u16,               // 02
    heads : u16,                  // 03 heads
    reserved2 : [u16 ; 5 - 3],        // 05
    sectors : u16,                // 06 sectors per track
    reserved3 : [u16; 9 - 6],        // 09
    serial : [u8; 20],              // 10 ~ 19 序列号
    reserved4 : [u16; 22 - 19],      // 10 ~ 22
    firmware : [u8; 8],             // 23 ~ 26 固件版本
    model : [u8; 40],               // 27 ~ 46 模型数
    drq_sectors : u8,             // 47 扇区数量
    reserved5 : [u8; 3],             // 48
    capabilities : u16,           // 49 能力
    reserved6 : [u16; 59 - 49],      // 50 ~ 59
    total_lba : u32,              // 60 ~ 61
    reserved7 : u16,               // 62
    mdma_mode : u16,              // 63
    reserved8 : u8,                // 64
    pio_mode : u8,                // 64
    reserved9 : [u16; 79 - 64],      // 65 ~ 79 参见 ATA specification
    major_version : u16,          // 80 主版本
    minor_version : u16,           // 81 副版本
    commmand_sets : [u16; 87 - 81], // 82 ~ 87 支持的命令集
    reserved10 : [u16; 118 - 87],     // 88 ~ 118
    support_settings : u16,       // 119
    enable_settings : u16,        // 120
    reserved11 : [u16; 221 - 120],    // 221
    transport_major : u16,        // 222
    transport_minor : u16,        // 223
    reserved12 : [u16; 254 - 223],    // 254
    integrity : u16              // 校验和
}


#[inline]
pub fn inb(port : u16) -> u8
{
    let mut result : u8;
    unsafe 
    {
        asm!(
        "in al, dx",
        out("al") result,
        in("dx") port,
        );
    }
    return result;
}

#[inline]
pub fn inw(port : u16) -> u16
{
    let mut result : u16;
    unsafe 
    {
        asm!(
        "in ax, dx",
        out("ax") result,
        in("dx") port,
        );
    }
    return result;
}

#[inline]
pub fn outb(port : u16, value : u8)
{
    unsafe
    {
        asm!(
            "out dx, al",
            in("al") value,
            in("dx") port,
        )
    }
}

#[inline]
pub fn outw(port : u16, value : u16)
{
    unsafe
    {
        asm!(
            "out dx, ax",
            in("ax") value,
            in("dx") port,
        )
    }
}


unsafe impl Sync for IdeDiskT {
    
}

impl IdePart {
    const fn empty() -> Self
    {
        Self { name: [0; 8], disk: null_mut(), system: 0, start: 0, count: 0 }
    }
}

#[derive(Clone, Copy)]
pub struct IdePart
{
    name : [c_char; 8],
    disk : *mut IdeDiskT,
    system : u32,
    start : u32,
    count : u32
}

pub struct IdeDiskT
{
    name : [char; 8],                  // 磁盘名称
    ctrl : *mut IdeCtrlT,       // 控制器指针
    selector : u8,                   // 磁盘选择
    master : bool,                   // 主盘
    total_lba : u32,                 // 可用扇区数量
    cylinders : u32,
    heads : u32,
    sectors : u32,
    lock : SpinLock,
    parts : [IdePart; IDE_PART_NR] // 硬盘分区
}

impl IdeDiskT {
    pub const fn empty() -> Self
    {
        Self { name: ['\0'; 8], ctrl: null_mut(), selector: 0, master: false, total_lba: 0, cylinders: 0, heads: 0, sectors: 0, lock: SpinLock::new(1), parts: [IdePart::empty(); IDE_PART_NR] }
    }

    pub fn new(ctrl_block : *mut IdeCtrlT, disk_selector : u8, is_master : bool, lba_num : u32, heads : u32, cylinders : u32, sectors : u32) -> IdeDiskT
    {
        IdeDiskT
        {
            name : ['\0'; 8],
            ctrl : ctrl_block,
            selector : disk_selector,
            master : is_master,
            total_lba : lba_num,
            cylinders,
            heads,
            sectors,
            lock: SpinLock::new(1),
            parts: [IdePart::empty(); IDE_PART_NR],
        }
    }
}

unsafe impl Sync for IdeCtrlT {
    
}

pub struct IdeCtrlT
{
    pub name : [char; 8],
    pub lock : semaphore::SpinLock,
    pub control : u8,
    pub iobase : u16,
    pub disks : [IdeDiskT; IDE_DISK_NR],
    pub active : *const IdeDiskT
}

impl IdeCtrlT {
    pub const fn new() -> IdeCtrlT
    {
        IdeCtrlT
        {
            name : ['\0'; 8],
            lock : semaphore::SpinLock::new(1),
            iobase : 0,
            control: 0,
            disks: [IdeDiskT::empty(), IdeDiskT::empty()],
            active: null(),
        }
    }
}

fn ide_select_drive(disk : &IdeDiskT)
{
    unsafe {
        io::outb((*disk.ctrl).iobase + IDE_HDDEVSEL, disk.selector);
        (*disk.ctrl).active = disk
    }
}
#[inline]
fn ide_busy_wait(ctrl : *mut IdeCtrlT, mask : u8)
{
    unsafe
    {
        loop {
            let state = inb((*ctrl).iobase + IDE_ALT_STATUS);
            if state & IDE_SR_ERR != 0
            {
                ide_error(&*ctrl);
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
}

fn ide_select_sector(disk : &IdeDiskT, lba : u64, cnt : u8)
{
    unsafe
    {
        outb((*disk.ctrl).iobase + IDE_FEATURE, 0);
        outb((*disk.ctrl).iobase + IDE_SECTOR, cnt);
        outb((*disk.ctrl).iobase + IDE_LBA_LOW, (lba & 0xff) as u8);
        outb((*disk.ctrl).iobase + IDE_LBA_MID, (lba >> 8 & 0xff) as u8);
        outb((*disk.ctrl).iobase + IDE_LBA_HIGH, (lba >> 16 & 0xff) as u8);
        outb((*disk.ctrl).iobase + IDE_HDDEVSEL, (lba >> 24 & 0xf) as u8 | (*disk).selector);
        (*(*disk).ctrl).active = disk as *const IdeDiskT;
    }
}

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

fn ide_pio_read_sector(disk : &IdeDiskT, mut offset : *mut u16)
{
    let mut cnt = 0;
    unsafe
    {
        while cnt < SECTOR_SIZE / 2
        {
            *offset = inw((*(disk.ctrl)).iobase + IDE_DATA);
            offset = offset.offset(1);
            cnt += 1;
        }
    }
}

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

fn ide_swap_pairs(buf : *mut c_char, len : u32)
{
    unsafe
    {
        let mut i = 0;
        while i < len {
            let ch = *buf.offset(i as isize);
            *buf.offset(i as isize) = *buf.offset(i as isize + 1);
            *buf.offset(i as isize + 1) = ch;
            i += 2;
        }
        *buf.offset(len as isize - 1) = 0;
    }
}

fn ide_error(ctrl : &IdeCtrlT)
{
    let error = io::inb(ctrl.iobase + IDE_ERR);
    if (error & IDE_ER_BBK) != 0
    {
        logk!("bad block\n");
    }
    if (error & IDE_ER_UNC) != 0
    {
        logk!("uncorrectable data\n");
    }
    if (error & IDE_ER_MC) != 0
    {
        logk!("media change\n");
    }
    if (error & IDE_ER_IDNF) != 0
    {
        logk!("id not found\n");
    }
    if (error & IDE_ER_MCR) != 0
    {
        logk!("media change requested\n");
    }
    if (error & IDE_ER_ABRT) != 0
    {
        logk!("abort\n");
    }
    if (error & IDE_ER_TK0NF) != 0
    {
        logk!("track 0 not found\n");
    }
    if (error & IDE_ER_AMNF) != 0
    {
        logk!("address mark not found\n");
    }
}

fn ide_identify(disk : &mut IdeDiskT) -> i64
{
    unsafe
    {
        disk.lock.acquire(1);
        ide_select_drive(disk);
        outb((*disk.ctrl).iobase + IDE_COMMAND, IDE_CMD_IDENTIFY);
        ide_busy_wait(disk.ctrl, IDE_SR_NULL);
        let params = memory::MEMORY_POOL.alloc(Layout::new::<IdeParamsT>()) as *mut IdeParamsT;
        ide_pio_read_sector(disk, params as *mut u16);
        let lba_num = (*params).total_lba;
        printk!("disk {} taotal lba number: {}\n", CStr::from_ptr(disk.name.as_ptr() as *const i8).to_str().unwrap(), lba_num);
        let mut ret = EOF;
        if (*params).total_lba == 0
        {
            (*disk.ctrl).lock.release(1);
            return ret;
        }
        ide_swap_pairs((*params).serial.as_mut_ptr() as *mut i8, 20);
        logk!("disk {} serial number {}\n", CStr::from_ptr(disk.name.as_ptr() as *mut i8).to_str().unwrap(), CStr::from_ptr((*params).serial.as_ptr() as *mut i8).to_str().unwrap());
        ide_swap_pairs((*params).firmware.as_mut_ptr() as *mut i8, 8);
        logk!("disk {} firmware version {}\n", CStr::from_ptr(disk.name.as_ptr() as *mut i8).to_str().unwrap(), CStr::from_ptr((*params).firmware.as_ptr() as *mut i8).to_str().unwrap());
        ide_swap_pairs((*params).model.as_mut_ptr() as *mut i8, 40);
        logk!("disk {} model number {}\n", CStr::from_ptr(disk.name.as_ptr() as *mut i8).to_str().unwrap(), CStr::from_ptr((*params).model.as_ptr() as *mut i8).to_str().unwrap());
        disk.total_lba = (*params).total_lba;
        disk.cylinders = (*params).cylinders as u32;
        disk.heads = (*params).heads as u32;
        disk.sectors = (*params).sectors as u32;
        ret = 0;
        MEMORY_POOL.dealloc(params as *mut u8, Layout::new::<IdeParamsT>());
        ret
    }
}

const IDE_PART_NR: usize = 4;
unsafe fn ide_part_init(disk : &mut IdeDiskT)
{
    if disk.total_lba == 0
    {
        return;
    }
    let buf = MEMORY_POOL.alloc(Layout::from_size_align_unchecked(4096, 1));
    ide_pio_sync_read(disk, 0, 1, buf);
    let boot = buf as *const BootSector;
    let mut var = 0;
    while var < IDE_PART_NR {
        let entry = &(*boot).entry[var] as *const PartEntry;
        let ptr = disk as *mut IdeDiskT;
        let part = &mut disk.parts[var];
        if (*entry).count == 0
        {
            var += 1;
            continue;
        }
        let mut str = String::new();
        let _ = core::fmt::write(&mut str, format_args!("{}{}", CStr::from_ptr(disk.name.as_ptr() as *mut i8).to_str().unwrap(), var + 1));
        assert!(str.len() < 8);
        compiler_builtins::mem::memcpy(part.name.as_ptr() as *mut u8, str.as_ptr(), str.len());
        logk!("part: {}\n", CStr::from_ptr(part.name.as_ptr() as *const c_char).to_str().unwrap());
        logk!("    bootable {}\n", (*entry).bootable);
        logk!("    start {}\n", (*entry).start);
        logk!("    count {}\n", (*entry).count);
        logk!("    system {:#x}\n", (*entry).system);
        part.disk = ptr;
        part.count = (*entry).count;
        part.system = (*entry).system as u32;
        part.start = (*entry).count;
        var += 1;
        if part.system == PART_FS_EXTENDED
        {
            panic!("unspport extended partition!!!");
        }
    }
    
}

pub fn ide_ctrl_init()
{
    unsafe
    {
        let mut cidx = 0;
        while cidx < IDE_CTRL_NR {
            let ctrl = &mut CONTROLLERS[cidx];

            if cidx != 0
            {
                *ctrl = IdeCtrlT::new();
                ctrl.iobase = IDE_IOBASE_SECONDARY;
            }
            else {
                *ctrl = IdeCtrlT::new();
                ctrl.iobase = IDE_IOBASE_PRIMARY;
            }
            (*ctrl).control = inb(ctrl.iobase + IDE_CONTROL);

            let mut didx = 0;
            while didx < IDE_DISK_NR
            {
                let ctrl_ptr = ctrl as *mut IdeCtrlT;
                let disk = &mut ctrl.disks[didx];
                let mut str = String::new(); 
                let _ = core::fmt::write(&mut str, format_args!("hd{}", ('a' as usize + cidx * 2 + didx) as u8 as char));
                compiler_builtins::mem::memcpy(disk.name.as_mut_ptr() as *mut u8, str.as_ptr(), str.len());
                disk.ctrl = ctrl_ptr;
                if didx == 0
                {
                    (*disk).master = true;
                    (*disk).selector = IDE_LBA_MASTER;
                }
                else {
                    (*disk).master = false;
                    (*disk).selector = IDE_LBA_SLAVE;
                }
                ide_identify(disk);
                ide_part_init(disk);
                didx += 1;
            }
            cidx += 1;
        }
    }
}

#[inline]
fn ide_early_busy_wait(io_base : u16 ,mask : u8)
{
    unsafe
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
}

pub fn ide_early_pio_sync_read(start_block : u32, num_blocks : u8, dst : *mut u8)
{
    let mut var = 0u64;
    if num_blocks <= 0
    {
        panic!("read blocks can't lower than 1");
    }
    else
    {
        unsafe 
        {
            io::outb(IDE_IOBASE_PRIMARY + IDE_HDDEVSEL, IDE_LBA_MASTER);
            ide_early_busy_wait(IDE_IOBASE_PRIMARY, IDE_SR_DRDY);
            ide_early_select_sector(IDE_IOBASE_PRIMARY, IDE_LBA_MASTER, start_block as u64, num_blocks);
            outb(IDE_IOBASE_PRIMARY + IDE_COMMAND, IDE_CMD_READ);
            while var < num_blocks as u64 {
                ide_early_busy_wait(IDE_IOBASE_PRIMARY, IDE_SR_DRQ);
                ide_early_pio_read_sector(IDE_IOBASE_PRIMARY, (dst as u64 + SECTOR_SIZE * var) as *mut u16);
                var += 1;
            }
        }
    }
}

pub fn ide_pio_sync_read(disk : &IdeDiskT, start_block : u32, num_blocks : u8, dst : *mut u8)
{
    let mut var = 0u64;
    if num_blocks <= 0
    {
        panic!("read blocks can't lower than 1");
    }
    else
    {
        unsafe 
        {
            ide_select_drive(disk);
            ide_busy_wait(disk.ctrl, IDE_SR_DRDY);
            ide_select_sector(disk, start_block as u64, num_blocks);
            outb((*(disk.ctrl)).iobase + IDE_COMMAND, IDE_CMD_READ);
            while var < num_blocks as u64 {
                ide_busy_wait(disk.ctrl, IDE_SR_DRQ);
                ide_pio_read_sector(disk, (dst as u64 + SECTOR_SIZE * var) as *mut u16);
                var += 1;
            }
        }
    }
}
