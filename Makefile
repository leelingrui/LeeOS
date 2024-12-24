BUILD:=./build
KERNEL_SRC:=./kernel/src
KERNEL_FILES:=$(KERNEL_SRC)/lib.rs $(KERNEL_SRC)/kernel/console.rs $(KERNEL_SRC)/kernel/interrupt.rs $(KERNEL_SRC)/kernel/io.rs $(KERNEL_SRC)/kernel/lang_items.rs \
	$(KERNEL_SRC)/kernel/mod.rs $(KERNEL_SRC)/kernel/relocation.rs $(KERNEL_SRC)/kernel/semaphore.rs $(KERNEL_SRC)/kernel/string.rs $(KERNEL_SRC)/kernel/interrupt.asm \
	$(KERNEL_SRC)/kernel/entry.asm $(KERNEL_SRC)/kernel/global.rs $(KERNEL_SRC)/mm/memory.rs $(KERNEL_SRC)/kernel/bitmap.rs $(KERNEL_SRC)/kernel/clock.rs $(KERNEL_SRC)/kernel/math.rs \
	$(KERNEL_SRC)/kernel/process.rs $(KERNEL_SRC)/mm/slub.rs $(KERNEL_SRC)/kernel/list.rs $(KERNEL_SRC)/mm/page.rs $(KERNEL_SRC)/kernel/fpu.rs  \
	$(KERNEL_SRC)/kernel/cpu.rs $(KERNEL_SRC)/kernel/bitops.rs $(KERNEL_SRC)/kernel/syscall.rs $(KERNEL_SRC)/fs/namei.rs $(KERNEL_SRC)/kernel/elf64.rs\
	$(KERNEL_SRC)/fs/file.rs $(KERNEL_SRC)/fs/mod.rs $(KERNEL_SRC)/mm/mm_type.rs $(KERNEL_SRC)/kernel/sched.rs $(KERNEL_SRC)/fs/ntfs.rs $(KERNEL_SRC)/kernel/time.rs \
	$(KERNEL_SRC)/fs/ext4.rs $(KERNEL_SRC)/fs/super_block.rs $(KERNEL_SRC)/kernel/device.rs $(KERNEL_SRC)/kernel/buffer.rs $(KERNEL_SRC)/kernel/execve.rs $(KERNEL_SRC)/kernel/fork.rs \
	$(KERNEL_SRC)/kernel/keyboard.rs $(KERNEL_SRC)/kernel/rtc.rs $(KERNEL_SRC)/kernel/input.rs $(KERNEL_SRC)/mm/shmem.rs $(KERNEL_SRC)/kernel/errno_base.rs $(KERNEL_SRC)/fs/dcache.rs $(KERNEL_SRC)/fs/fs.rs\
	$(KERNEL_SRC)/fs/mnt_idmapping.rs $(KERNEL_SRC)/fs/libfs.rs $(KERNEL_SRC)/fs/fs_context.rs $(KERNEL_SRC)/fs/path.rs $(KERNEL_SRC)/fs/ns_common.rs $(KERNEL_SRC)/fs/ida.rs \
	$(KERNEL_SRC)/fs/mount.rs
MACRO_SRC:=./proc_macro/src
MACRO_FILES:=$(MACRO_SRC)/lib.rs $(MACRO_SRC)/__init.rs $(MACRO_SRC)/__exit.rs

ENTRYPOINT:=0x0xffff800000100000
# RFLAGS+= target-feature=-crt-static
RFLAGS:=$(strip ${RFLAGS})
DEBUG:=


BUILTIN_APP=$(BUILD)/x86_64-unknown-leeos/debug/init


LIB_SRC:=./lib/src
LIB_FILES:=$(LIB_SRC)/lib.rs $(LIB_SRC)/unistd.rs $(LIB_SRC)/macros.rs $(LIB_SRC)/print.rs ./lib/Makefile $(LIB_SRC)/../.cargo/config.toml $(LIB_SRC)/lang_items.rs \

	

BUILTIN_SRC:=./builtins/src
BUILTIN_APP_FILES:=$(BUILTIN_SRC)/bin/init.rs $(BUILTIN_SRC)/lib.rs $(BUILTIN_SRC)/lang_items.rs


$(BUILD)/boot/%.asm.bin: $(KERNEL_SRC)/boot/%.asm
	$(shell mkdir -p $(dir $@))
	nasm -f bin $(DEBUG) $< -o $@

$(BUILD)/kernel/%.asm.bin: $(KERNEL_SRC)/kernel/%.asm
	$(shell mkdir -p $(dir $@))
	nasm -f bin $(DEBUG) $< -o $@

$(BUILD)/x86_64-unknown-leeos/debug/liblib.rlib: $(LIB_FILES) 
	$(MAKE) -C ./lib build_lib

$(BUILD)/x86_64-unknown-leeos/debug/liblib.so: $(LIB_FILES) 
	$(MAKE) -C ./lib build_lib

$(BUILTIN_APP): $(BUILTIN_APP_FILES) $(BUILD)/x86_64-unknown-leeos/debug/liblib.so
	$(MAKE) -C ./builtins build_builtins

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

$(BUILD)/x86_64-unknown-none/debug/lee_os: $(KERNEL_FILES) $(MACRO_FILES) ./kernel/Makefile ./kernel/Cargo.toml $(KERNEL_SRC)/linker.ld
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
	-rtc base=utc \
	-audiodev wav,id=hda \
	-machine pcspk-audiodev=hda \
	-chardev stdio,mux=on,id=com1 \
	-serial chardev:com1 \
	-vnc 0:0

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

.PHONY: check
check:
	$(MAKE) -C ./kernel check_kernel
	$(MAKE) -C ./lib check_lib
	$(MAKE) -C ./builtins check_builtins

.PHONY: clean
clean:
	rm -rf $(BUILD)
