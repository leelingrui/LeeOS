OUTPUT_ARCH(x86_64)
ENTRY(_start)
BASE_ADDRESS = 0xffff800000100000;

PHDRS
{
    text PT_LOAD;
    data PT_LOAD ;
    rodata PT_LOAD ;
    bss PT_LOAD ;
    init PT_LOAD ;
    dynamic PT_DYNAMIC ;
}


SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    } : text

    . = ALIGN(8);
    .got : {
        *(.got)
    } : text

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    } : data

    . = ALIGN(4K);
    etext = .;
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    } : rodata

    . = ALIGN(8);

    .data.rel.ro : 
    {
        *(.data.rel.ro)
    } : rodata

    . = ALIGN(4K);
    edata = .;
    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    } : bss

    . = ALIGN(4K);
    .init.text : {
        *(.init.*)
    } : init

    . = ALIGN(4K);
    .rela.dyn : {
        *(.rela.dyn)
    } : init

    . = ALIGN(4K);
    dyn = .;

    . = ALIGN(4K);
    .dynamic : {
        *(.dynamic)
    } : dynamic

    .dynsym :
    {
        *(.dynsym)
    } : dynamic
    .gnu.hash : {
        *(.gnu.hash)
    } : dynamic
    .hash :
    {
        *(.hash)
    } : dynamic
    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}