use core::{arch::{asm, global_asm}, default, fmt};
use bitfield::{bitfield, size_of};

use crate::printk;

const PIC_M_CTRL : u16 = 0x20; // 主片的控制端口
const PIC_M_DATA : u16 =  0x21; // 主片的数据端口
const PIC_S_CTRL : u16 =  0xa0; // 从片的控制端口
const PIC_S_DATA : u16 =  0xa1; // 从片的数据端口
const PIC_EOI : u8 =  0x20;    // 通知中断控制器中断结束

const FAULT_MESSAGES : [&str; 22] = [
    "#DE Divide Error",
    "#DB RESERVED",
    "--  NMI Interrupt",
    "#BP Breakpoint",
    "#OF Overflow",
    "#BR BOUND Range Exceeded",
    "#UD Invalid Opcode (Undefined Opcode)",
    "#NM Device Not Available (No Math Coprocessor)",
    "#DF Double Fault",
    "    Coprocessor Segment Overrun (reserved)",
    "#TS Invalid TSS",
    "#NP Segment Not Present",
    "#SS Stack-Segment Fault",
    "#GP General Protection",
    "#PF Page Fault",
    "--  (Intel reserved. Do not use.)",
    "#MF x87 FPU Floating-Point Error (Math Fault)",
    "#AC Alignment Check",
    "#MC Machine Check",
    "#XF SIMD Floating-Point Exception",
    "#VE Virtualization Exception",
    "#CP Control Protection Exception",
];

use super::io::outb;
const IDT_SIZE : usize = 0x100;
static mut IDT : [DescriptorT; IDT_SIZE] = [DescriptorT(0); IDT_SIZE];
#[no_mangle]
static mut IDT_PTR : PointerT = PointerT{ base: 0, limit: 0 };
const SYS_CALL_RESERVED_SIZE : usize = 16;
type HandlerFn = *const extern fn();
#[no_mangle]
pub static mut HANDLER_TABLE : [HandlerFn; IDT_SIZE] = [core::ptr::null(); IDT_SIZE];
extern
{
    static mut handler_entry_table : [HandlerFn; IDT_SIZE];
}

global_asm!(include_str!("./interupt.asm"));

bitfield!
{
    #[derive(Clone, Copy)]
    pub struct DescriptorT(u128);
    u64;
    get_low_offset, set_low_offset : 15, 0;
    get_selector, set_selector : 31, 16;
    get_zero1, _ : 39, 32;
    get_type, set_type : 43, 40;
    get_reserved, _ : 44, 44;
    get_dpl, set_dpl : 46, 45;
    get_present, set_present : 47, 47;
    get_high_offset, set_high_offset : 95, 48;
    get_zero2, _ : 127, 96;
}

impl fmt::Display for DescriptorT {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "offset low: {:#4x};\nselector: {:#16b};\ntype: {:#5b};\ndpl: {:#2b};\npresent: {:#};\noffset high: {:#4x};",
    self.get_low_offset(), self.get_selector(), self.get_type(), self.get_dpl(), self.get_present(), self.get_high_offset())
    }
}

fn init_8259a()
{
    outb(PIC_M_CTRL, 0b00010001); // ICW1: 边沿触发, 级联 8259, 需要ICW4.
    outb(PIC_M_DATA, 0x20);       // ICW2: 起始中断向量号 0x20
    outb(PIC_M_DATA, 0b00000100); // ICW3: IR2接从片.
    outb(PIC_M_DATA, 0b00000001); // ICW4: 8086模式, 正常EOI

    outb(PIC_S_CTRL, 0b00010001); // ICW1: 边沿触发, 级联 8259, 需要ICW4.
    outb(PIC_S_DATA, 0x28);       // ICW2: 起始中断向量号 0x28
    outb(PIC_S_DATA, 2);          // ICW3: 设置从片连接到主片的 IR2 引脚
    outb(PIC_S_DATA, 0b00000001); // ICW4: 8086模式, 正常EOI

    outb(PIC_M_DATA, 0b11111111); // 关闭所有中断
    outb(PIC_S_DATA, 0b11111111); // 关闭所有中断
}

pub fn get_idt(no : isize) -> DescriptorT
{
    let mut idt_pointer = unsafe { IDT_PTR.clone() };
    let dst = &mut idt_pointer as *mut PointerT as u64;
    unsafe
    {
        asm!(
            "sidt [{gdt_ptr}]",
            gdt_ptr = in(reg) dst
        );
        let local_gdt = *((idt_pointer.base as *mut DescriptorT).offset(no));
        local_gdt
    }
}

struct PtRegs
{
    r15 : u64,
    r14 : u64,
    r13 : u64,
    r12 : u64,
    rbp : u64,
    rbx : u64,
    // always save
    r11 : u64,
    r10 : u64,
    r9 : u64,
    r8 : u64,
    rax : u64,
    rcx : u64,
    rdx : u64,
    rsi : u64,
    rdi : u64,
    orig_ax : u64,
    rip : u64,
    cs : u64,
    flags : u64,
    rsp : u64,
    ss : u64
    // top of stack
}
fn default_handler(vector : u32)
{
    printk!("[{}] default interrupt called...\n", vector);
    send_eoi(vector);
}

pub fn regist_irq(irq_func : *const extern fn(), interrupt_no : u8)
{
    unsafe {
        HANDLER_TABLE[interrupt_no as usize] = irq_func;
    }
}

fn send_eoi(vector : u32)
{
    if vector >= 0x20 && vector < 0x28
    {
        outb(PIC_M_CTRL, PIC_EOI);
    }
    if vector >= 0x28 && vector < 0x30
    {
        outb(PIC_M_CTRL, PIC_EOI);
        outb(PIC_S_CTRL, PIC_EOI);
    }
}

impl DescriptorT
{
    fn descriptor_init(&mut self, offset : u64, selector : u16, type_t : u8, dpl : u8, present : bool)
    {
        self.set_low_offset(offset & 0xffff);
        self.set_selector(selector as u64);
        self.set_type(type_t as u64);
        self.set_dpl(dpl as u64);
        self.set_present(present as u64);
        self.set_high_offset((offset >> 16) & 0xffffffffffff);
    }
}

#[repr(C)]
#[repr(packed)]
#[derive(Default, Clone)]
pub struct PointerT
{
    limit : u16,
    base : u64
}

fn exception_handler(vector : u32)
{
    let mut message = "";
    if vector < 22
    {
        message = FAULT_MESSAGES[vector as usize];
    }
    printk!("\nEXCEPTION{}\n", message);
}

fn idt_init()
{
    unsafe
    {
        let mut var = 0;
        while var < SYS_CALL_RESERVED_SIZE
        {
            IDT[var].descriptor_init(handler_entry_table[var] as u64, 1 << 3, 0b1110, 0, true);
            var += 1;
        }
        var = 0;
        while var < 0x20 {
            regist_irq(exception_handler as *const extern fn(), var as u8);
            var += 1;
        }
        IDT[80].descriptor_init(default_handler as u64, 1 << 3, 0b1110, 0, true);
        IDT_PTR.base = IDT.as_ptr() as u64;
        IDT_PTR.limit = (IDT_SIZE * 16 - 1) as u16;
        asm!("lidt [IDT_PTR]");
        set_interrupt_state(true);
    }
}

pub fn interrupt_disable() -> bool
{
    unsafe
    {
        let interrupt_status : u64;
        asm!(
            "pushf",
            "cli",
            "pop rax",
            out("rax") interrupt_status
        );
        interrupt_status & 0x20 != 0
    }
}

pub fn interrupt_init()
{
    init_8259a();
    idt_init();
}

pub fn set_interrupt_state(state : bool)
{
    unsafe
    {
        if state
        {
            asm!("sti");
        }
        else {
            asm!("cli")
        }
    }
}

pub fn get_interrupt_state() -> bool
{
    let result : u64;
    unsafe
    {
        asm!(
            "pushf
            pop rax
            test rax, 0x100",
            out("rax") result
        )
    }
    return  result != 0;
}