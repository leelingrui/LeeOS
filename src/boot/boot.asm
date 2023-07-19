[org 0x7c00]

; 设置屏幕模式为文本模式. 清除屏幕
    mov ax, 3
    int 0x10

    ; 初始化段寄存器
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7c00
    ; 0xb8000 文本显示器的内存区域
    mov ds, ax
    mov ax, 0xb800
    mov si, booting
    call print
    mov edi, 0x1000; 读取目标地址
    mov ecx, 2; 起始扇区
    mov bl, 4; 扇区数量
    call read_disk
    ; 阻塞
    mov si, readdisk_fin
    call print
    cmp word [0x1000], 0x55aa
    jnz error
    jmp 0:0x1002
    jmp $

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

write_disk:
    ;设置读写扇区的数量
    mov dx, 0x1f2
    mov al, bl
    out dx,al

    inc dx;
    mov al, cl;
    out dx, al

    inc dx;
    shr ecx, 8
    mov al, cl
    out dx, al
    inc dx;
    shr ecx, 8
    mov al, cl
    out dx, al
    
    inc dx
    shr ecx, 8
    and cl, 0b111; 将高四位置为0

    mov al, 0b1110_0000;
    or al, cl
    out dx, al

    inc dx
    mov al, 0x20;   写硬盘
    out dx, al

    xor ecx, ecx
    mov cl, bl; 得到读写扇区的数量

.write:
    push cx
    call .waits;
    call .writes;
    pop cx
    loop .write
    ret

.waits:
    mov dx, 0x1f7
.check:
    in al, dx
    jmp $+2
    jmp $+2
    jmp $+2
    and al, 0b1000_0000
    cmp al, 0b0000_0000
    jnz .check
    ret

.writes:
    mov dx, 0x1f0
    mov cx, 256
.writew:
    mov ax, [edi]
    out dx, ax
    add edi, 2
    jmp $+2
    jmp $+2
    jmp $+2
    loop .writew
    ret

error:
    mov si, .msg
    mov ah, 0x0e
    mov al, [0x1000]
    int 0x10
    mov al, [0x1001]
    int 0x10
    call print
    hlt
    jmp $
.msg: 
    db "Booting Error!", 10, 13, 0
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

booting:
    db "Booting LeeOS...", 10, 13, 0; \n\r
readdisk_fin:
    db "Read Disk Finished...", 10, 13, 0
times 510 - ($ - $$) db 0
; 主引导扇区最后两个字节必须是0x55 0xaa
db 0x55, 0xaa
