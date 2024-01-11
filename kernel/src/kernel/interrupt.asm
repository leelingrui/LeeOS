.globl _syscall_start
.globl interrupt_exit
.globl _syscall_end

.macro SAVE_CONTEXT
    sub rsp, 8 * 16
    mov [rsp + 15 * 8], rax
    mov [rsp + 14 * 8], rcx
    mov [rsp + 13 * 8], rdx
    mov [rsp + 12 * 8], rbx
    mov [rsp + 11 * 8], rsi
    mov [rsp + 10 * 8], rdi
    mov [rsp + 9 * 8], rbp
    mov [rsp + 8 * 8], r8
    mov [rsp + 7 * 8], r9
    mov [rsp + 6 * 8], r10
    mov [rsp + 5 * 8], r11
    mov [rsp + 4 * 8], r12
    mov [rsp + 3 * 8], r13
    mov [rsp + 2 * 8], r14
    mov [rsp + 1 * 8], r15
    mov [rsp + 0 * 8 + 6], ds
    mov [rsp + 0 * 8 + 4], es
    mov [rsp + 0 * 8 + 2], fs
    mov [rsp + 0 * 8 + 0], gs
.endm

.macro RECOVER_CONTEXT
    mov rax, [rsp + 15 * 8]
    mov rcx, [rsp + 14 * 8]
    mov rdx, [rsp + 13 * 8]
    mov rbx, [rsp + 12 * 8]
    mov rsi, [rsp + 11 * 8]
    mov rdi, [rsp + 10 * 8]
    mov rbp, [rsp + 9 * 8]
    mov r8, [rsp + 8 * 8]
    mov r9, [rsp + 7 * 8]
    mov r10, [rsp + 6 * 8]
    mov r11, [rsp + 5 * 8]
    mov r12, [rsp + 4 * 8]
    mov r13, [rsp + 3 * 8]
    mov r14, [rsp + 2 * 8]
    mov r15, [rsp + 1 * 8]
    mov ds, [rsp + 0 * 8 + 6]
    mov es, [rsp + 0 * 8 + 4]
    mov fs, [rsp + 0 * 8 + 2]
    mov gs, [rsp + 0 * 8 + 0]
    add rsp, 8 * 16
.endm


.macro INTERRUPT_HANDLER NAME:req, TYPE:req
interrupt_handler_\NAME\():
    xchg bx, bx
.ifeq \TYPE
    push 0x20000906
.endif
    push \NAME
    jmp interrupt_entry
.endm
interrupt_entry:
    SAVE_CONTEXT
    lea rsi, [rsp]
    mov rdi, [rsp + 16 * 8]
    mov rax, [HANDLER_TABLE@GOTPCREL + rip]
    mov rax, [rax + rdi * 8]
    call rax
interrupt_exit:
    // call task signal
    RECOVER_CONTEXT
    add rsp, 0x10
    xchg bx, bx
    iretq

_syscall_start:
    sub rsp, 8 * 16
    mov [rsp + 15 * 8], rax
    mov [rsp + 14 * 8], rcx
    mov [rsp + 13 * 8], rdx
    mov [rsp + 12 * 8], rbx
    mov [rsp + 11 * 8], rsi
    mov [rsp + 10 * 8], rdi
    mov [rsp + 8 * 8], r8
    mov [rsp + 7 * 8], r9
    mov [rsp + 6 * 8], r10
    mov [rsp + 5 * 8], r11
    mov [rsp + 4 * 8], r12
    mov [rsp + 3 * 8], r13
    mov [rsp + 2 * 8], r14
    mov [rsp + 1 * 8], r15
    mov [rsp + 0 * 8], rsp
    lea rdi, [rsp]
    call [syscall_function@GOTPCREL + rip]
_syscall_end:
    mov rax, [rsp + 15 * 8]
    mov rcx, [rsp + 14 * 8]
    mov rbx, [rsp + 12 * 8]
    mov rbp, [rsp + 9 * 8]
    mov r11, [rsp + 5 * 8]
    mov r12, [rsp + 4 * 8]
    mov r13, [rsp + 3 * 8]
    mov r14, [rsp + 2 * 8]
    mov r15, [rsp + 1 * 8]
    mov rsp, [rsp + 0 * 8]
    add rsp, 8 * 16
    xchg bx, bx
    sysretq

INTERRUPT_HANDLER 0x00, 0
INTERRUPT_HANDLER 0x01, 0
INTERRUPT_HANDLER 0x02, 0
INTERRUPT_HANDLER 0x03, 0
INTERRUPT_HANDLER 0x04, 0
INTERRUPT_HANDLER 0x05, 0
INTERRUPT_HANDLER 0x06, 0
INTERRUPT_HANDLER 0x07, 0
INTERRUPT_HANDLER 0x08, 1
INTERRUPT_HANDLER 0x09, 0
INTERRUPT_HANDLER 0x0a, 1
INTERRUPT_HANDLER 0x0b, 1
INTERRUPT_HANDLER 0x0c, 1
INTERRUPT_HANDLER 0x0d, 1
INTERRUPT_HANDLER 0x0e, 1
INTERRUPT_HANDLER 0x0f, 0
INTERRUPT_HANDLER 0x10, 0
INTERRUPT_HANDLER 0x11, 1
INTERRUPT_HANDLER 0x12, 0
INTERRUPT_HANDLER 0x13, 0
INTERRUPT_HANDLER 0x14, 0
INTERRUPT_HANDLER 0x15, 1
INTERRUPT_HANDLER 0x16, 0
INTERRUPT_HANDLER 0x17, 0
INTERRUPT_HANDLER 0x18, 0
INTERRUPT_HANDLER 0x19, 0
INTERRUPT_HANDLER 0x1a, 0
INTERRUPT_HANDLER 0x1b, 0
INTERRUPT_HANDLER 0x1c, 0
INTERRUPT_HANDLER 0x1d, 0
INTERRUPT_HANDLER 0x1e, 0
INTERRUPT_HANDLER 0x1f, 0
INTERRUPT_HANDLER 0x20, 0
INTERRUPT_HANDLER 0x21, 0
INTERRUPT_HANDLER 0x22, 0
INTERRUPT_HANDLER 0x23, 0
INTERRUPT_HANDLER 0x24, 0
INTERRUPT_HANDLER 0x25, 0
INTERRUPT_HANDLER 0x26, 0
INTERRUPT_HANDLER 0x27, 0
INTERRUPT_HANDLER 0x28, 0
INTERRUPT_HANDLER 0x29, 0
INTERRUPT_HANDLER 0x2a, 0
INTERRUPT_HANDLER 0x2b, 0
INTERRUPT_HANDLER 0x2c, 0
INTERRUPT_HANDLER 0x2d, 0
INTERRUPT_HANDLER 0x2e, 0
INTERRUPT_HANDLER 0x2f, 0






.section .data
handler_entry_table:
    .quad interrupt_handler_0x00
    .quad interrupt_handler_0x01
    .quad interrupt_handler_0x02
    .quad interrupt_handler_0x03
    .quad interrupt_handler_0x04
    .quad interrupt_handler_0x05
    .quad interrupt_handler_0x06
    .quad interrupt_handler_0x07
    .quad interrupt_handler_0x08
    .quad interrupt_handler_0x09
    .quad interrupt_handler_0x0a
    .quad interrupt_handler_0x0b
    .quad interrupt_handler_0x0c
    .quad interrupt_handler_0x0d
    .quad interrupt_handler_0x0e
    .quad interrupt_handler_0x0f
    .quad interrupt_handler_0x10
    .quad interrupt_handler_0x11
    .quad interrupt_handler_0x12
    .quad interrupt_handler_0x13
    .quad interrupt_handler_0x14
    .quad interrupt_handler_0x15
    .quad interrupt_handler_0x16
    .quad interrupt_handler_0x17
    .quad interrupt_handler_0x18
    .quad interrupt_handler_0x19
    .quad interrupt_handler_0x1a
    .quad interrupt_handler_0x1b
    .quad interrupt_handler_0x1c
    .quad interrupt_handler_0x1d
    .quad interrupt_handler_0x1e
    .quad interrupt_handler_0x1f
    .quad interrupt_handler_0x20
    .quad interrupt_handler_0x21
    .quad interrupt_handler_0x22
    .quad interrupt_handler_0x23
    .quad interrupt_handler_0x24
    .quad interrupt_handler_0x25
    .quad interrupt_handler_0x26
    .quad interrupt_handler_0x27
    .quad interrupt_handler_0x28
    .quad interrupt_handler_0x29
    .quad interrupt_handler_0x2a
    .quad interrupt_handler_0x2b
    .quad interrupt_handler_0x2c
    .quad interrupt_handler_0x2d
    .quad interrupt_handler_0x2e
    .quad interrupt_handler_0x2f