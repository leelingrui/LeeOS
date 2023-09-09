BUILD:=./build
SRC:=./src
RSFILES:=$(SRC)/main.rs $(SRC)/kernel/console.rs $(SRC)/kernel/interrupt.rs $(SRC)/kernel/io.rs $(SRC)/kernel/lang_items.rs \
	$(SRC)/kernel/mod.rs $(SRC)/kernel/relocation.rs $(SRC)/kernel/semaphore.rs $(SRC)/kernel/string.rs $(SRC)/kernel/interrupt.asm \
	$(SRC)/kernel/entry.asm $(SRC)/kernel/global.rs $(SRC)/kernel/memory.rs $(SRC)/kernel/bitmap.rs $(SRC)/kernel/clock.rs $(SRC)/kernel/math.rs \
	$(SRC)/kernel/process.rs $(SRC)/kernel/slub.rs $(SRC)/kernel/list.rs $(SRC)/kernel/page.rs $(SRC)/kernel/fpu.rs  \
	$(SRC)/kernel/cpu.rs $(SRC)/kernel/bitops.rs
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


$(BUILD)/master.img: $(BUILD)/boot/boot.asm.bin \
	$(BUILD)/boot/loader.asm.bin \
	$(BUILD)/x86_64-unknown-none/debug/lee_os $(BUILD)/system.map
	yes | bximage -q -hd=16 -func=create -sectsize=512 -imgmode=flat $@
	dd if=$(BUILD)/boot/boot.asm.bin of=$@ bs=512 count=1 conv=notrunc
	dd if=$(BUILD)/boot/loader.asm.bin of=$@ bs=512 count=4 seek=2 conv=notrunc
	dd if=$(BUILD)/x86_64-unknown-none/debug/lee_os of=$@ bs=512 seek=10 conv=notrunc

.PHONY: qemug
qemug: $(BUILD)/master.img
	qemu-system-x86_64 -s -S -m 32M -boot c \
	-drive file=$<,if=ide,index=0,media=disk,format=raw \
	-audiodev wav,id=hda \
	-machine pcspk-audiodev=hda

.PHONY: qemu
qemu: $(BUILD)/master.img
	qemu-system-x86_64 -m 32M -boot c \
	-drive file=$<,if=ide,index=0,media=disk,format=raw \
	-audiodev wav,id=hda \
	-machine pcspk-audiodev=hda
.PHONY: bochs
bochs: $(BUILD)/master.img
	bochs -q -f bochsrc -unlock

.PHONY: bochsg
bochsg: $(BUILD)/master.img
	bochs-gdb -q -f bochsrc.gdb -unlock

.PHONY: clean
clean:
	rm -rf $(BUILD)