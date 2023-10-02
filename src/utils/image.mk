$(BUILD)/master.img: $(BUILD)/boot/boot.asm.bin \
	$(BUILD)/boot/loader.asm.bin \
	$(SRC)/utils/master.sfdisk \
	$(BUILD)/x86_64-unknown-none/debug/lee_os $(BUILD)/system.map
# 创建磁盘镜像
	yes | bximage -q -hd=16 -func=create -sectsize=512 -imgmode=flat $@
	dd if=$(BUILD)/boot/boot.asm.bin of=$@ bs=512 count=1 conv=notrunc
	dd if=$(BUILD)/boot/loader.asm.bin of=$@ bs=512 count=4 seek=2 conv=notrunc
	dd if=$(BUILD)/x86_64-unknown-none/debug/lee_os of=$@ bs=512 seek=10 conv=notrunc

	sfdisk $@ < $(SRC)/utils/master.sfdisk
	sudo losetup /dev/loop0 --partscan $@

	sudo mkfs.ext4 -n 255 /dev/loop0p1

	sudo mount /dev/loop0p1 /mount

	sudo chown ${USER} /mnt

	mkdir -p /mnt/bin
	mkdir -p /mnt/dev
	mkdir -p /mnt/mnt


	sudo umount /mnt
	sudo losetup -d /dev/loop0

$(BUILD)/slave.img: $(SRC)/utils/slave.sfdisk

# 创建一个 32M 的硬盘镜像
	yes | bximage -q -hd=32 -func=create -sectsize=512 -imgmode=flat $@

# 执行硬盘分区
	sfdisk $@ < $(SRC)/utils/slave.sfdisk

# 挂载设备
	sudo losetup /dev/loop0 --partscan $@

# 创建 minux 文件系统
	sudo mkfs.ext4 /dev/loop0p1

# 挂载文件系统
	sudo mount /dev/loop0p1 /mnt

# 切换所有者
	sudo chown ${USER} /mnt 

# 创建文件
	echo "slave root direcotry file..." > /mnt/hello.txt

# 卸载文件系统
	sudo umount /mnt

# 卸载设备
	sudo losetup -d /dev/loop0

IMAGES:= $(BUILD)/master.img $(BUILD)/slave.img
image: $(IMAGES)
