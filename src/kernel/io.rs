use core::arch::asm;

use super::interrupt;
use super::semaphore;
use super::io;

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

pub struct IdeDiskT
{
    name : [char;8],                  // 磁盘名称
    ctrl : *mut IdeCtrlT,       // 控制器指针
    selector : u8,                   // 磁盘选择
    master : bool,                   // 主盘
    total_lba : u32                 // 可用扇区数量
    // ide_part_t parts[IDE_PART_NR]; // 硬盘分区
}

impl IdeDiskT {
    pub fn new(ctrl_block : *mut IdeCtrlT, disk_selector : u8, is_master : bool, lba_num : u32) -> IdeDiskT
    {
        IdeDiskT
        {
            name : ['\0'; 8],
            ctrl : ctrl_block,
            selector : disk_selector,
            master : is_master,
            total_lba : lba_num
        }
    }
}


pub struct IdeCtrlT
{
    name : [char; 8],
    lock : semaphore::SpinLock,
    iobase : u16,
}

impl IdeCtrlT {
    pub fn new(iobase : u16) -> IdeCtrlT
    {
        IdeCtrlT
        {
            name : ['\0'; 8],
            lock : semaphore::SpinLock::new(1),
            iobase
        }
    }
}

fn ide_select_drive(disk : &IdeDiskT)
{
    unsafe {
        io::outb((*disk.ctrl).iobase + IDE_HDDEVSEL, disk.selector);
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

fn ide_select_sector(disk : &IdeDiskT, lba : u32, cnt : u8)
{
    unsafe
    {
        outb((*disk.ctrl).iobase + IDE_FEATURE, 0);
        outb((*disk.ctrl).iobase + IDE_SECTOR, cnt);
        outb((*disk.ctrl).iobase + IDE_LBA_LOW, (lba & 0xff) as u8);
        outb((*disk.ctrl).iobase + IDE_LBA_MID, (lba >> 8 & 0xff) as u8);
        outb((*disk.ctrl).iobase + IDE_LBA_HIGH, (lba >> 16 & 0xff) as u8);
        outb((*disk.ctrl).iobase + IDE_HDDEVSEL, (lba >> 24 & 0xf) as u8 | (*disk).selector);
        // (*(*disk).ctrl).
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

pub fn ide_pio_sync_read(disk : &IdeDiskT, start_block : u32, num_blocks : u8, dst : *mut u8)
{
    let mut var = 0u64;
    if num_blocks <= 0
    {
        panic!()
    }
    else
    {
        unsafe 
        {
            ide_select_drive(disk);
            ide_busy_wait(disk.ctrl, IDE_SR_DRDY);
            ide_select_sector(disk, start_block, num_blocks);
            outb((*(disk.ctrl)).iobase + IDE_COMMAND, IDE_CMD_READ);
            while var < num_blocks as u64 {
                ide_busy_wait(disk.ctrl, IDE_SR_DRQ);
                ide_pio_read_sector(disk, (dst as u64 + SECTOR_SIZE * var) as *mut u16);
                var += 1;
            }
        }
    }
}
