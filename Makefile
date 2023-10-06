BUILD:=./build
SRC:=./src
RSFILES:=$(SRC)/main.rs $(SRC)/kernel/console.rs $(SRC)/kernel/interrupt.rs $(SRC)/kernel/io.rs $(SRC)/kernel/lang_items.rs \
	$(SRC)/kernel/mod.rs $(SRC)/kernel/relocation.rs $(SRC)/kernel/semaphore.rs $(SRC)/kernel/string.rs $(SRC)/kernel/interrupt.asm \
	$(SRC)/kernel/entry.asm $(SRC)/kernel/global.rs $(SRC)/mm/memory.rs $(SRC)/kernel/bitmap.rs $(SRC)/kernel/clock.rs $(SRC)/kernel/math.rs \
	$(SRC)/kernel/process.rs $(SRC)/mm/slub.rs $(SRC)/kernel/list.rs $(SRC)/mm/page.rs $(SRC)/kernel/fpu.rs  \
	$(SRC)/kernel/cpu.rs $(SRC)/kernel/bitops.rs $(SRC)/kernel/syscall.rs $(SRC)/lib/mod.rs $(SRC)/lib/unistd.rs \
	$(SRC)/fs/file.rs $(SRC)/fs/mod.rs $(SRC)/mm/mm_type.rs $(SRC)/kernel/sched.rs $(SRC)/fs/ntfs.rs $(SRC)/kernel/time.rs \
	$(SRC)/fs/ext4.rs $(SRC)/fs/super_block.rs $(SRC)/kernel/device.rs
ENTRYPOINT:=0x0xffff800000100000
# RFLAGS+= target-feature=-crt-static
RFLAGS:=$(strip ${RFLAGS})
DEBUG:=

$(BUILD)/boot/%.asm.bin: $(SRC)/boot/%.asm
	$(shell mkdir -p $(dir $@))
	nasm -f bin $(DEBUG) $< -o $@

$(BUILD)/kernel/%.asm.bin: $(SRC)/kernel/%.asm
	$(shell mkdir -p $(dir $@))
	nasm -f bin $(DEBUG) $< -o $@

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

$(BUILD)/x86_64-unknown-none/debug/lee_os: $(RSFILES) \
											$(SRC)/linker.ld
	cargo $(DEBUG) build

$(BUILD)/system.bin: $(BUILD)/x86_64-unknown-none/debug/lee_os
	objcopy -O binary $< $@

$(BUILD)/system.map: $(BUILD)/x86_64-unknown-none/debug/lee_os
	nm $< | sort > $@


include $(SRC)/utils/image.mk

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