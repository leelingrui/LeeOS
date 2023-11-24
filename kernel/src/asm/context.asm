.macro SAVE_CONTEXT
    sub rsp, 8 * 18
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
    mov rcx, [rsp + 0 * 8]
    mov rdx, [rsp + 0 * 8]
    mov rbx, [rsp + 0 * 8]
    mov rsi, [rsp + 0 * 8]
    mov rdi, [rsp + 0 * 8]
    mov rsp, [rsp + 0 * 8]
    mov rbp, [rsp + 0 * 8]
    mov rip, [rsp + 0 * 8]
    mov r8, [rsp + 0 * 8]
    mov r9, [rsp + 0 * 8]
    mov r10, [rsp + 0 * 8]
    mov r11, [rsp + 0 * 8]
    mov r12, [rsp + 0 * 8]
    mov r13, [rsp + 0 * 8]
    mov r14, [rsp + 0 * 8]
    mov r15, [rsp + 0 * 8]
    mov ds, [rsp + 17 * 8 + 0]
    mov es, [rsp + 17 * 8 + 0]
    mov fs, [rsp + 17 * 8 + 0]
    mov gs, [rsp + 17 * 8 + 0]
    add rsp, 8 * 18
.endm