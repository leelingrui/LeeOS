.section .text.entry
.globl _start
_start:
    mov rdi, 0x20000
    call kernel_relocation
    call kernel_init
    xchg bx, bx
    int 0
    hlt