use core::{ffi::c_void, arch::asm, fmt, default::Default};

use bitfield::bitfield;

use crate::printk;

const ARDS_BUFFER : *const c_void = 0x7c00 as *const c_void;
const KERNEL_PAGE_DIR : *const c_void = 0x200000 as *const c_void;
const PAGE_SIZE : usize = 0x200000;
static mut MEMORY_DESCRIPTOR : MemoryDescriptor = MemoryDescriptor{ start : core::ptr::null(), size : 0, all_pages : 0};

struct MemoryDescriptor
{
    start : *const c_void,
    size : usize,
    all_pages : usize
}

#[repr(packed)]
struct E820Map
{
    addr : u64,
    size : u64,
    memory_type : u32
}

impl fmt::Display for E820Map
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mem_type = self.memory_type;
        let addr = self.addr;
        let size = self.size;
        write!(f, "Address: {:#x},\tLength: {:#x},\tType: {:#x}\n", addr, size, mem_type)
    }
}

bitfield!
{
    struct Pml4Entry(u64);
    u64;
    _, set_always : 0, 0;
    get_wr, set_wr : 1, 1;
    get_us, set_us : 2, 2;
    get_pwt, set_pwt : 3, 3;
    get_pcd, set_pcd : 4, 4;
    get_accessed, set_accessed : 5, 5;
    get_reserved, _ : 11, 6;
    get_pdpt_offset, set_pdpt_offset : 63, 12;
}

#[inline]
fn get_inpage_offset(ptr : *const c_void) -> u64
{
    ptr as u64 & 0xfffff
}

#[inline]
fn get_pdpt_offset(ptr : *const c_void) -> u64
{
    (ptr as u64 >> 30) & 0xff
}

#[inline]
fn get_pml4_offset(ptr : *const c_void) -> u64
{
    (ptr as u64 >> 38) & 0x3ff
}

#[no_mangle]
pub unsafe fn alloc_page(num : usize) -> *const u8
{
    core::ptr::null::<u8>()
}

#[inline]
fn set_cr3_reg(pml4_ptr : *const c_void)
{
    unsafe { asm!(
            "mov cr3, {_pml4_ptr}",
            _pml4_ptr = in(reg) pml4_ptr
        ) };
}

#[inline]
fn get_cr3_reg() -> u64
{
    let mut cr3_reg : u64;
    unsafe { asm!("mov {cr3}, cr3",
            cr3 = out(reg) cr3_reg 
        ) };
    cr3_reg
}

fn print_ards(mut e820map_addr : *const E820Map)
{
    unsafe
    {
        loop {
            if (*e820map_addr).memory_type > 4
            {
                break;
            }
            else {
                printk!("{}", (*e820map_addr))
            }
            e820map_addr = e820map_addr.offset(1);
        }
    }

}



unsafe fn get_useable_memory(descriptor : *const E820Map)
{
    if (*descriptor).size as usize > MEMORY_DESCRIPTOR.size
    {
        MEMORY_DESCRIPTOR.size = (*descriptor).size as usize;
        MEMORY_DESCRIPTOR.start = (*descriptor).addr as *const c_void;
    }
}

fn get_page_size()
{
    unsafe
    {
        MEMORY_DESCRIPTOR.all_pages = MEMORY_DESCRIPTOR.size / PAGE_SIZE;
    }
}

pub fn init_memory(magic : u32, address : *const c_void)
{
    let mut e820map_addr : *mut E820Map = ARDS_BUFFER as *mut E820Map;
    print_ards(e820map_addr);
    unsafe
    {
        e820map_addr = e820map_addr.offset(1);
        loop {
            match (*e820map_addr).memory_type {
                1 => {
                    get_useable_memory(e820map_addr);
                    e820map_addr = e820map_addr.offset(1);
                    continue;
                },
                2 | 3 => {
                    e820map_addr = e820map_addr.offset(1);
                    continue;
                },
                _ => break
            } 
        }
        get_page_size();
        printk!("total page num: {}\n", MEMORY_DESCRIPTOR.all_pages);
    }
}