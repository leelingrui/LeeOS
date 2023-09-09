.section .text.entry
.globl _start
_start:
    mov rsp, 0xffff800000090000
    mov rbp, 0
    mov rdi, 0xffff800000020000
    call kernel_relocation
    call kernel_init
    xchg bx, bx
    hlt
    flag:
    jmp flag