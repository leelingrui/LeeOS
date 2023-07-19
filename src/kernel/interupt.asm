.macro SAVE_CONTEXT
    sub rsp, 8 * 17
    mov [rsp + 0 * 8], rax
    mov [rsp + 1 * 8], rcx
    mov [rsp + 2 * 8], rdx
    mov [rsp + 3 * 8], rbx
    mov [rsp + 4 * 8], rsi
    mov [rsp + 5 * 8], rdi
    mov [rsp + 6 * 8], rsp
    mov [rsp + 7 * 8], rbp
    mov [rsp + 8 * 8], r8
    mov [rsp + 9 * 8], r9
    mov [rsp + 10 * 8], r10
    mov [rsp + 11 * 8], r11
    mov [rsp + 12 * 8], r12
    mov [rsp + 13 * 8], r13
    mov [rsp + 14 * 8], r14
    mov [rsp + 15 * 8], r15
    mov [rsp + 16 * 8 + 0], ds
    mov [rsp + 16 * 8 + 2], es
    mov [rsp + 16 * 8 + 4], fs
    mov [rsp + 16 * 8 + 6], gs
.endm

.macro RECOVER_CONTEXT
    mov rax, [rsp + 0 * 8]
    mov rcx, [rsp + 1 * 8]
    mov rdx, [rsp + 2 * 8]
    mov rbx, [rsp + 3 * 8]
    mov rsi, [rsp + 4 * 8]
    mov rdi, [rsp + 5 * 8]
    mov rsp, [rsp + 6 * 8]
    mov rbp, [rsp + 7 * 8]
    mov r8, [rsp + 8 * 8]
    mov r9, [rsp + 9 * 8]
    mov r10, [rsp + 10 * 8]
    mov r11, [rsp + 11 * 8]
    mov r12, [rsp + 12 * 8]
    mov r13, [rsp + 13 * 8]
    mov r14, [rsp + 14 * 8]
    mov r15, [rsp + 15 * 8]
    mov ds, [rsp + 16 * 8 + 0]
    mov es, [rsp + 16 * 8 + 0]
    mov fs, [rsp + 16 * 8 + 0]
    mov gs, [rsp + 16 * 8 + 0]
    add rsp, 8 * 17
.endm



.macro INTERRUPT_HANDLER NAME:req, TYPE:req
interrupt_handler_\NAME\():
.ifne \TYPE
    push 0x20000906
.endif
    push \NAME
    jmp interrupt_entry
.endm
interrupt_entry:
    SAVE_CONTEXT
    mov rdi, [rsp + 17 * 8]
    call [HANDLER_TABLE + rax * 8]
interrupt_exit:
    // call task signal
    RECOVER_CONTEXT
    iret


INTERRUPT_HANDLER 0x00, 0
INTERRUPT_HANDLER 0x01, 0
INTERRUPT_HANDLER 0x02, 0
INTERRUPT_HANDLER 0x03, 0
INTERRUPT_HANDLER 0x04, 0
INTERRUPT_HANDLER 0x05, 0
INTERRUPT_HANDLER 0x06, 0
INTERRUPT_HANDLER 0x07, 0
INTERRUPT_HANDLER 0x08, 0
INTERRUPT_HANDLER 0x09, 0
INTERRUPT_HANDLER 0x0a, 0
INTERRUPT_HANDLER 0x0b, 0
INTERRUPT_HANDLER 0x0c, 0
INTERRUPT_HANDLER 0x0d, 0
INTERRUPT_HANDLER 0x0e, 0
INTERRUPT_HANDLER 0x0f, 0


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

