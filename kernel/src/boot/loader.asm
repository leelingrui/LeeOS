[org 0x1000]
code_selector equ (1 << 3)
data_selector equ (2 << 3)
elf_header_pos equ 0x20000
ards_buffer equ 0x7c00
memory_base equ 0; 内存开始的位置: 基地址
; 内存界限 4G / 4K - 1
memory_limit equ ((1024 * 1024 * 1024 * 4) / (1024 * 4)) - 1

dw 0x55aa

detect_memory:
    xor ebx, ebx
    xor ax, ax
    mov es, ax
    mov edi, ards_buffer
    mov edx, 0x534d4150
.next:
    mov eax, 0xe820
    mov ecx, 20
    int 0x15
    jc error
    add di, cx
    inc word [ards_count]
    cmp ebx, 0
    jnz .next
    mov si, detecting
    call print
prepare_protected_mode:
    cli
    in al, 0x92
    or al, 0b10
    out 0x92, al
    lgdt [gdt_ptr]
    lidt [IDT_POINTER]
    mov eax, cr0
    or eax, 1
    mov cr0, eax 
    jmp dword code_selector:protect_mode



[bits 32]
protect_mode:
    call support_long_mode
    test eax, eax
    jnz long_mode
    mov ax, data_selector
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax; 初始化段寄存器
    mov esp, 0x10000
    mov edi, 0x10000
    mov ecx, 10
    mov bl, 200
    call read_disk
    jmp code_selector:0x10000

read_disk:
    ;设置读写扇区的数量
    mov dx, 0x1f2
    mov al, bl
    out dx, al

    inc dx
    mov al, cl
    out dx, al

    inc dx
    shr ecx, 8
    mov al, cl
    out dx, al

    inc dx
    shr ecx, 8
    mov al, cl
    out dx, al
    
    inc dx
    shr ecx, 8
    and cl, 0b1111; 将高四位置为0

    mov al, 0b1110_0000;
    or al, cl
    out dx, al

    inc dx
    mov al, 0x20;   读硬盘
    out dx, al

    xor ecx, ecx
    mov cl, bl; 得到读写扇区的数量
    cmp bl, 0
    jne .read
    mov cx, 0x100
.read:
    push cx
    call .waits
    call .reads
    pop cx
    loop .read
    ret

.waits:
    mov dx, 0x1f7
.check:
    in al, dx
    jmp $+2
    jmp $+2
    jmp $+2
    and al, 0b1000_1000
    cmp al, 0b0000_1000
    jnz .check
    ret

.reads:
    mov dx, 0x1f0
    mov cx, 256
.readw:
    in ax, dx
    mov [edi], ax
    jmp $+2
    jmp $+2
    jmp $+2
    add edi, 2
    loop .readw
    ret
support_long_mode:

	mov	eax,	0x80000000
	cpuid
	cmp	eax,	0x80000001
	setnb	al	
	jb	support_long_mode_done
	mov	eax,	0x80000001
	cpuid
	bt	edx,	29
	setc	al
support_long_mode_done:
	
	movzx	eax,	al
	ret

long_mode:
    mov ax, data_selector
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax; 初始化段寄存器
    mov esp, 0x90000
    call load_system64_header
	mov	dword	[0x90000],	0x91007
	mov	dword	[0x90800],	0x93007
    mov dword   [0x90880],  0x93007
	mov	dword	[0x91000],	0x92007
	mov	dword	[0x92000],	0x000083
	mov	dword	[0x92008],	0x200083
	mov	dword	[0x92010],	0x400083
	mov	dword	[0x92018],	0x600083
	mov	dword	[0x92020],	0x800083
	mov	dword	[0x92028],	0xa00083
	mov	dword	[0x92030],	0xc00083
	mov	dword	[0x92038],	0xe00083
    mov dword   [0x93000],  0x94007
    mov dword   [0x94000],  0x000083
    mov dword   [0x94008],  0x200083
    mov dword   [0x94010],  0x400083
    mov dword   [0x94018],  0x600083
    mov dword   [0x94020],  0x800083
    mov dword   [0x94028],  0xa00083
    mov dword   [0x94030],  0xc00083
    mov dword   [0x94038],  0xe00083
    mov dword   [0x94040],  0x1000083
    mov dword   [0x94048],  0x1200083
    mov dword   [0x94050],  0x1400083
    mov dword   [0x94058],  0x1600083
    mov dword   [0x94060],  0x1800083
    mov dword   [0x94068],  0x1a00083
    mov dword   [0x94070],  0x1c00083
    mov dword   [0x94078],  0x1e00083
	; mov	dword	[0x92040],	0x1000083
	; mov	dword	[0x92048],	0x1200083
    mov	eax,	cr4
	bts	eax,	5
	bts	eax,	4
	mov	cr4,	eax
    mov	eax,	0x90000
	mov	cr3,	eax
    mov ecx, 0xc0000080
    rdmsr
    bts eax, 0x8
    wrmsr
    mov eax, cr0
    bts eax, 0
    bts eax, 31
    mov cr0, eax
    lgdt [gdt_ptr64]
    jmp code_selector:jmp_dst
jmp_dst:
[BITS 64]
    mov rax, 0xffff800000100000
    call rax
[BITS 32]

chech_elf_64:
    push edi
    mov esi, elf_magic
    mov dx, 0x10
    call str_cmp ;check Magic Num
    test eax, eax
    jz .check_elf_faile 
    pop edi
    add edi, 0x10
    mov ax, [edi]
    cmp ax, 0x3 ;check File Type
    jne .check_elf_faile
    add edi, 0x2
    mov ax, [edi]
    cmp ax, 0x3e ;chech Target Machine
    jne .check_elf_faile
    mov eax, 1
    ret
.check_elf_faile:
    mov eax, 0
    ret

str_cmp:
    mov cx, dx
.start_cmp:
    mov al, [esi]
    cmp al, [edi]
    jne .not_equ
    inc esi
    inc edi
    loop .start_cmp
    mov eax, 1
    ret
.not_equ:
    mov eax, 0
    ret   

elf_magic:
    db 0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00

load_code_segment:
    mov eax, esi
    add eax, 0x20
    mov ebx, [eax]; Load Phdr Address Offset
    add ebx, esi; Load Phdr Address
    mov eax, [ebx]
    cmp eax, 0x01; Check Code Segment Type Correct
    jne .code_segment_error
    mov eax, ebx
    add eax, 0x4
    mov ecx, dword [eax]
    test ecx, 0x101; Check Code Segment Flags Correct
    jz .code_segment_error
    add eax, 0x4
    mov eax, [eax]
    mov edx, 0
    push ebx
    mov ebx, 512
    div ebx
    mov ecx, eax
    add ecx, 0xa; 计算从何处开始读取磁盘块
    pop ebx
    push ecx
    mov eax, ebx
    add eax, 40
    mov eax, [eax]
    mov ebx, 512
    div ebx
    mov ebx, eax
    cmp edx, 0
    je .next2
    inc ebx; 计算读取多少磁盘块
.next2:
    pop ecx
    test ebx, 0x100
    push ebx; read block num
    push ecx; start block
    push edi; dst position
    call read_disk
    pop edi
    pop ecx
    pop ebx
.read_block:
    movzx esi, bl
    add ecx, esi
    shl esi, 9
    add edi, esi
    and bl, 0
    cmp ebx, 0
    je .read_finish
    sub ebx, 0x100
    push ebx; read block num
    push ecx; start block
    push edi; dst position
    call read_disk
    pop edi
    pop ecx
    pop ebx
    cmp ebx, 0
    add ecx, 0x100
    add edi, 0x100 * 512
    jmp .read_block
.read_finish
    mov eax, 1
    ret

.code_segment_error:
    mov eax, 0
    ret
    

load_system64_header:
    mov edi, elf_header_pos
    mov ecx, 10
    mov bl, 8
    call read_disk
    mov edi, elf_header_pos
    call chech_elf_64
    test eax, eax
    jz error
    mov edi, [elf_header_pos + 0x18]
    mov esi, elf_header_pos
    call load_code_segment 
    ret
gdt_ptr:
    dw (gdt_end - gdt_base) - 1
    dd gdt_base
align 8
gdt_base:
    dd 0, 0
gdt_code:
    dw memory_limit & 0xffff
    dw memory_base & 0xffff
    db (memory_base >> 16) & 0xff
    db 0b10011010
    db 0b11000000 | (memory_limit >> 16)
    db (memory_base >> 24) & 0xff
gdt_data:
    dw memory_limit & 0xffff
    dw memory_base & 0xffff
    db (memory_base >> 16) & 0xff
    db 0b10010010
    db 0b10100000 | (memory_limit >> 16)
    db (memory_base >> 24) & 0xff
gdt_end:

gdt_ptr64:
    dw (gdt_end64 - gdt_base64) - 1
    dd gdt_base64
align 8
gdt_base64:
    dq 0
gdt_code64: dq	0x0020980000000000
    ; dw memory_limit & 0xffff
    ; dw memory_base & 0xffff
    ; db (memory_base >> 16) & 0xff
    ; db 0b10011010
    ; db 0b1110000 | (memory_limit >> 16)
    ; db (memory_base >> 24) & 0xff
gdt_data64: dq 0x0000920000000000
    ; dw memory_limit & 0xffff
    ; dw memory_base & 0xffff
    ; db (memory_base >> 16) & 0xff
    ; db 0b10010010
    ; db 0b10100000 | (memory_limit >> 16)
    ; db (memory_base >> 24) & 0xff
gdt_end64:

; gdt_base:
;     dd 0, 0
; gdt_code:
;     dw memory_limit & 0xffff
;     dw memory_base & 0xffff
;     db (memory_base >> 16) & 0xff
;     db 0b10011010
;     db 0b11000000 | (memcry_limit >> 16)
;     db (memory_base >> 2c) & 0xff
; gdt_data:
;     dw memory_limit & 0xcfff
;     dw memory_base & 0xfcff
;     db (memory_base >> 1c) & 0xff
;     db 0b10010010
;     db 0b11000000 | (memcry_limit >> 16)
;     db (memory_base >> 2c) & 0xff
; gdt_end:

loading32:
    db "Loading LeeOS32", 10, 13, 0
loading64:
    db "Loading LeeOS64", 10, 13, 0
detecting:
    db "Detecting Memory Success...", 13, 10, 0
open_long_mode:
    db "Opening Long Mode...", 13, 10, 0
open_long_mode_success:
    db "Opening Long Mode Success...", 13, 10, 0
support_1G_big_page_msg:
    db "Support 1GB Big Page", 13, 10, 0

error:
    mov si, .msg
    call print
    jmp $

.msg: 
    db "Loading Error!", 10, 13, 0
[BITS 16]
print:
    mov ah, 0x0e
.next:
    mov al, [si]
    cmp al, 0
    jz .done
    int 0x10
    inc si
    jmp .next
.done:
    ret
ards_count:
    dw 0

IDT:
	times	0x50	dq	0
IDT_END:

IDT_POINTER:
		dw	IDT_END - IDT - 1
		dd	IDT


