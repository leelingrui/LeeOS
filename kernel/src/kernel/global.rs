use core::{intrinsics::size_of, default::Default, arch::asm, fmt};
use crate::{printk, kernel::string::memset};

const GDT_SIZE : usize = 7;
pub static mut GDT : [DescriptorT; GDT_SIZE] = [DescriptorT(0); GDT_SIZE];
pub static mut KERNEL_TSS : TaskStateSegment = TaskStateSegment::new();
#[no_mangle]
pub static mut GDT_PTR : PointerT = PointerT{ base: 0, limit: 0 };
use bitfield::bitfield;
pub const KERNEL_CODE_IDX : usize = 1;
pub const KERNEL_DATA_IDX : usize = 2;
pub const USER_DATA_IDX : usize = 3;
pub const USER_CODE_IDX : usize = 4;
pub const TSS_IDX : usize = 5;
#[repr(C)]
#[derive(Default, Clone)]
#[repr(packed)]
pub struct PointerT
{
    limit : u16,
    base : u64
}
pub type TSS = TaskStateSegment; 

#[repr(C, packed)]
pub struct TaskStateSegment
{
    reserved0 : u32,
    rsp0 : u64,
    rsp1 : u64,
    rsp2 : u64,
    reserved1 : u64,
    ist1 : u64,
    ist2 : u64,
    ist3 : u64,
    ist4 : u64,
    ist5 : u64,
    ist6 : u64,
    ist7 : u64,
    reserved2 : u64,
    reserved3 : u16,
    iobp : u16
}

impl TaskStateSegment {
    const fn new() -> TaskStateSegment
    {
        TaskStateSegment { reserved0: 0, rsp0: 0, rsp1: 0, rsp2: 0, reserved1: 0, ist1: 0, ist2: 0, ist3: 0, ist4: 0, ist5: 0, ist6: 0, ist7: 0, reserved2: 0, reserved3: 0, iobp: 0 }
    }
}

bitfield!
{
    #[derive(Clone, Copy)]
    pub struct DescriptorT(u64);
    u64;
    get_limit_low, set_limit_low : 15, 0;
    get_base_low, set_base_low : 39, 16;
    get_type, set_type : 43, 40; // A RW DC E
    get_segment, set_segment : 44, 44;
    get_dpl, set_dpl : 46, 45;
    get_present, set_present : 47, 47;
    get_limit_high, set_limit_high : 51, 48;
    get_available, set_available : 52, 52;
    get_long_mode, set_long_mode : 53, 53;
    get_big, set_big : 54, 54;
    get_granularity, set_granularity : 55, 55;
    get_base_high, set_base_high : 63, 56;
}

impl fmt::Display for DescriptorT
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "limit low: {:#4x};\nbase low: {:#6x};\ntype: {:#4b};\nsegment: {:#b};\n\
        dpl: {:#x};\npresent: {:#b};\nlimit high: {:#x};\navaliable: {:#b};\nlong mode: {:#b};\n\
        big: {:#b};\ngranykarity: {:#b};\nbase high: {:#2x};\n", self.get_limit_low(), self.get_base_low(), self.get_type(),
        self.get_segment(), self.get_dpl(), self.get_present(), self.get_limit_high(), self.get_available(),
        self.get_long_mode(), self.get_big(), self.get_granularity(), self.get_base_high())
    }
}

fn descriptor_init(desc : &mut DescriptorT, base : u64, limit : u32, segment : bool, granularity : bool, big : bool, long_mode : bool, present : bool, dpl : u8, type_t : u8)
{
    desc.set_base_low(base & 0xffffff);
    desc.set_base_high(base >> 24 & 0xff);
    desc.set_limit_low((limit & 0xffff) as u64);
    desc.set_limit_high((limit >> 16 & 0xf) as u64);
    desc.set_segment(segment as u64);
    desc.set_granularity(granularity as u64);
    desc.set_big(big as u64);
    desc.set_long_mode(long_mode as u64);
    desc.set_present(present as u64);
    desc.set_dpl(dpl as u64);
    desc.set_type(type_t as u64);
}

pub fn get_gdt(no : isize) -> DescriptorT
{
    let mut gdt_pointer = unsafe { GDT_PTR.clone() };
    let dst = &mut gdt_pointer as *mut PointerT as u64;
    unsafe
    {
        asm!(
            "sgdt [{gdt_ptr}]",
            gdt_ptr = in(reg) dst
        );
        let local_gdt = *((gdt_pointer.base as *mut DescriptorT).offset(no));
        local_gdt
    }
}

pub fn set_tss64(tss_ptr : &mut TaskStateSegment, rsp0 : u64, rsp1 : u64, rsp2 : u64, ist1 : u64, ist2 : u64, ist3 : u64, ist4 : u64, ist5 : u64, ist6 : u64, ist7 : u64)
{
    tss_ptr.rsp0 = rsp0;
    tss_ptr.rsp1 = rsp1;
    tss_ptr.rsp2 = rsp2;
    tss_ptr.ist1 = ist1;
    tss_ptr.ist2 = ist2;
    tss_ptr.ist3 = ist3;
    tss_ptr.ist4 = ist4;
    tss_ptr.ist5 = ist5;
    tss_ptr.ist6 = ist6;
    tss_ptr.ist7 = ist7;
}

pub fn tss_init()
{
    unsafe
    {
        let interrupt_stack = 0xffff800000090000u64;// memory::MEMORY_POOL.alloc_frames(1).offset(memory::PAGE_SIZE as isize);
        descriptor_init(&mut GDT[TSS_IDX], &KERNEL_TSS as *const TaskStateSegment as u64, (size_of::<TaskStateSegment>() - 1) as u32, false, false, false, false, true, 0, 0x9);
        set_tss64(&mut KERNEL_TSS, interrupt_stack as u64, interrupt_stack as u64, interrupt_stack as u64, interrupt_stack as u64, interrupt_stack as u64, interrupt_stack as u64, interrupt_stack as u64, interrupt_stack as u64, interrupt_stack as u64, interrupt_stack as u64);
        GDT[TSS_IDX + 1].0 = (&KERNEL_TSS as *const TaskStateSegment as u64) >> 32 & 0xffffffff;
        asm!(
            "ltr ax",
            in("ax") (TSS_IDX << 3)
        )
    }
}

#[no_mangle]
pub fn gdt_init()
{
    printk!("init gdt!!!\n");
    unsafe {
        memset(GDT.as_ptr() as *mut u8, 0, GDT_SIZE * size_of::<DescriptorT>());
        descriptor_init(&mut GDT[KERNEL_CODE_IDX], 0x0, 0xfffff, true, true, false, true, true, 0, 0b1010);
        descriptor_init(&mut GDT[KERNEL_DATA_IDX], 0x0, 0xfffff, true, true, false, true, true, 0, 0b0010);

        descriptor_init(&mut GDT[USER_CODE_IDX], 0x0, 0xfffff, true, true, false, true, true, 3, 0b1010);
        descriptor_init(&mut GDT[USER_DATA_IDX], 0x0, 0xfffff, true, true, false, true, true, 3, 0b0010);
        GDT_PTR.base = GDT.as_ptr() as u64;
        GDT_PTR.limit = (GDT_SIZE * size_of::<DescriptorT>() - 1) as u16;
        asm!(
            "lgdt [{gdt_ptr}]",
            gdt_ptr = in(reg) &GDT_PTR as *const PointerT as u64
        );
    }
}