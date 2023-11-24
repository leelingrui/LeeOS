BUILD:=./build
KERNEL_SRC:=./kernel/src
KERNEL_FILES:=$(KERNEL_SRC)/lib.rs $(KERNEL_SRC)/kernel/console.rs $(KERNEL_SRC)/kernel/interrupt.rs $(KERNEL_SRC)/kernel/io.rs $(KERNEL_SRC)/kernel/lang_items.rs \
	$(KERNEL_SRC)/kernel/mod.rs $(KERNEL_SRC)/kernel/relocation.rs $(KERNEL_SRC)/kernel/semaphore.rs $(KERNEL_SRC)/kernel/string.rs $(KERNEL_SRC)/kernel/interrupt.asm \
	$(KERNEL_SRC)/kernel/entry.asm $(KERNEL_SRC)/kernel/global.rs $(KERNEL_SRC)/mm/memory.rs $(KERNEL_SRC)/kernel/bitmap.rs $(KERNEL_SRC)/kernel/clock.rs $(KERNEL_SRC)/kernel/math.rs \
	$(KERNEL_SRC)/kernel/process.rs $(KERNEL_SRC)/mm/slub.rs $(KERNEL_SRC)/kernel/list.rs $(KERNEL_SRC)/mm/page.rs $(KERNEL_SRC)/kernel/fpu.rs  \
	$(KERNEL_SRC)/kernel/cpu.rs $(KERNEL_SRC)/kernel/bitops.rs $(KERNEL_SRC)/kernel/syscall.rs \
	$(KERNEL_SRC)/fs/file.rs $(KERNEL_SRC)/fs/mod.rs $(KERNEL_SRC)/mm/mm_type.rs $(KERNEL_SRC)/kernel/sched.rs $(KERNEL_SRC)/fs/ntfs.rs $(KERNEL_SRC)/kernel/time.rs \
	$(KERNEL_SRC)/fs/ext4.rs $(KERNEL_SRC)/fs/super_block.rs $(KERNEL_SRC)/kernel/device.rs $(KERNEL_SRC)/kernel/buffer.rs
ENTRYPOINT:=0x0xffff800000100000
# RFLAGS+= target-feature=-crt-static
RFLAGS:=$(strip ${RFLAGS})
DEBUG:=
BUILTIN_APP=$(BUILD)/x86_64-unknown-none/init

LIB_SRC:=./lib/src
LIB_FILES:=$(LIB_SRC)/lib.rs $(LIB_SRC)/unistd.rs ./lib/Makefile

$(BUILD)/boot/%.asm.bin: $(KERNEL_SRC)/boot/%.asm
	$(shell mkdir -p $(dir $@))
	nasm -f bin $(DEBUG) $< -o $@

$(BUILD)/kernel/%.asm.bin: $(KERNEL_SRC)/kernel/%.asm
	$(shell mkdir -p $(dir $@))
	nasm -f bin $(DEBUG) $< -o $@

$(BUILTIN_APP): $(LIB_FILES)

.PHONY: test
test: $(BUILD)/master.img

.PHONY: usb
usb: $(BUILD)/boot/boot.asm.bin /dev/sdb
	sudo dd if=/dev/sdb of=tmp.bin bs=512 count=1 conv=notrunc
	cp tmp.bin usb.bin
	sudo rm tmp.bin
	dd if=(BUILD)/boot/boot.asm.bin of=usb.bin bs=446 count=1 conv=notrunc
	sudo dd if=usb.bin of=dev/sdb bs=512 count=1 conv=notrunc
	rm usb.bin

$(BUILD)/x86_64-unknown-none/debug/lee_os: $(KERNEL_FILES) \
											$(KERNEL_SRC)/linker.ld ./kernel/Makefile
	$(MAKE) -C ./kernel build_kernel

#

$(BUILD)/system.bin: $(BUILD)/x86_64-unknown-none/debug/lee_os
	objcopy -O binary $< $@

$(BUILD)/system.map: $(BUILD)/x86_64-unknown-none/debug/lee_os
	nm $< | sort > $@


include ./utils/image.mk

.PHONY: qemug
qemug:  $(IMAGES)
	qemu-system-x86_64 -s -S -m 32M -boot c \
	-drive file=$(BUILD)/master.img,if=ide,index=0,media=disk,format=raw \
	-drive file=$(BUILD)/slave.img,if=ide,index=1,media=disk,format=raw \
	-rtc base=localtime \
	-audiodev wav,id=hda \
	-machine pcspk-audiodev=hda \
	-chardev stdio,mux=on,id=com1 \
	-serial chardev:com1

.PHONY: qemu
qemu:  $(IMAGES)
	qemu-system-x86_64 -m 32M -boot c \
	-drive file=$(BUILD)/master.img,if=ide,index=0,media=disk,format=raw \
	-drive file=$(BUILD)/slave.img,if=ide,index=1,media=disk,format=raw \
	-rtc base=localtime \
	-audiodev wav,id=snd \
	-machine pcspk-audiodev=hda \
	-chardev stdio,mux=on,id=com1 \
	-serial chardev:com1
.PHONY: bochs
bochs:  $(IMAGES)
	bochs -q -f bochsrc -unlock

.PHONY: bochsg
bochsg: $(IMAGES)
	bochs-gdb -q -f bochsrc.gdb -unlock

.PHONY: clean
clean:
	rm -rf $(BUILD)