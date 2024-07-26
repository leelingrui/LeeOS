$(BUILD)/master.img: $(BUILD)/boot/boot.asm.bin \
	$(BUILD)/boot/loader.asm.bin \
	./utils/master.sfdisk \
	$(BUILD)/x86_64-unknown-none/debug/lee_os $(BUILD)/system.map \
	$(BUILTIN_APP) ./utils/image.mk
# 创建磁盘镜像
	yes | bximage -q -hd=128 -func=create -sectsize=512 -imgmode=flat $@
	dd if=$(BUILD)/boot/boot.asm.bin of=$@ bs=512 count=1 conv=notrunc
	dd if=$(BUILD)/boot/loader.asm.bin of=$@ bs=512 count=4 seek=2 conv=notrunc
	dd if=$(BUILD)/x86_64-unknown-none/debug/lee_os of=$@ bs=512 seek=10 conv=notrunc

	sfdisk $@ < ./utils/master.sfdisk
	sudo losetup /dev/loop5 --partscan $@

	sudo mkfs.ext4 -c /dev/loop5p1

	sudo mount /dev/loop5p1 /mnt/LeeOSDisk

	sudo chown ${USER} /mnt/LeeOSDisk

	mkdir -p /mnt/LeeOSDisk/bin
	mkdir -p /mnt/LeeOSDisk/dev
	mkdir -p /mnt/LeeOSDisk/mnt

	for app in $(BUILTIN_APP); \
	do \
		cp $$app /mnt/LeeOSDisk/bin; \
	done

	sudo umount /mnt/LeeOSDisk
	sudo losetup -d /dev/loop5

$(BUILD)/slave.img: ./utils/slave.sfdisk ./utils/image.mk

# 创建一个 32M 的硬盘镜像
	yes | bximage -q -hd=32 -func=create -sectsize=512 -imgmode=flat $@

# 挂载设备
	sudo losetup /dev/loop5 --partscan $@

# 创建 ext4 文件系统
	sudo mkfs.ext4 -c /dev/loop5

# 挂载文件系统
	sudo mount /dev/loop5 /mnt/LeeOSDisk

# 切换所有者
	sudo chown ${USER} /mnt/LeeOSDisk

# 创建文件
	echo "slave root direcotry file..." > /mnt/LeeOSDisk/hello.txt

# 卸载文件系统
	sudo umount /mnt/LeeOSDisk

# 卸载设备
	sudo losetup -d /dev/loop5

IMAGES:= $(BUILD)/master.img $(BUILD)/slave.img
image: $(IMAGES)
