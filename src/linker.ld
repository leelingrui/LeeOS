OUTPUT_ARCH(x86_64)
ENTRY(_start)
BASE_ADDRESS = 0x100000;

SECTIONS
{
    . = BASE_ADDRESS;
    skernel = .;

    stext = .;
    .text : {
        *(.text.entry)
        *(.text .text.*)
    }

    . = ALIGN(4K);
    .dynamic : {
        *(.dynamic)
    }
    
    . = ALIGN(4K);
    .got : {
        *(.got)
    }

    . = ALIGN(4K);
    erodata = .;
    sdata = .;
    .data : {
        *(.data .data.*)
        *(.sdata .sdata.*)
    }

    . = ALIGN(4K);
    etext = .;
    srodata = .;
    .rodata : {
        *(.rodata .rodata.*)
        *(.srodata .srodata.*)
    }

    . = ALIGN(4K);
    edata = .;
    .bss : {
        *(.bss.stack)
        sbss = .;
        *(.bss .bss.*)
        *(.sbss .sbss.*)
    }
    . = ALIGN(4K);
    dyn = .;
    .dynsym :
    {
        *(.dynsym)
    }
    .gnu.hash : {
        *(.gnu.hash)
    }
    .hash :
    {
        *(.hash)
    }
    . = ALIGN(4K);
    ebss = .;
    ekernel = .;

    /DISCARD/ : {
        *(.eh_frame)
    }
}