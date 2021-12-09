ARCH?=aarch64
ifeq ($(ARCH),aarch64)
MACH?=qemu
endif
GDB?=gdb-multiarch

LLVM_BASE=$(shell llvm-config --bindir)
CLANG=clang-14
LDLLD=ld.lld-12
OBJCOPY=$(LLVM_BASE)/llvm-objcopy
MKIMAGE?=mkimage

PROFILE?=debug
O=target/$(ARCH)-$(MACH)/$(PROFILE)

CARGO_COMMON_OPTS=
ifeq ($(PROFILE),release)
CARGO_COMMON_OPTS+=--release
endif
ifeq ($(VERBOSE),1)
CARGO_COMMON_OPTS+=--features verbose
endif

CARGO_BUILD_OPTS=$(CARGO_COMMON_OPTS) \
				 --target=../etc/$(ARCH)-$(MACH).json
ifneq ($(MACH),)
CARGO_BUILD_OPTS+=--features mach_$(MACH)
endif

QEMU_OPTS=-s
ifeq ($(ARCH),x86_64)
$(error TODO)
else
ifeq ($(MACH),qemu)
QEMU_OPTS+=-kernel $(O)/kernel.bin \
		   -initrd $(O)/initrd.img \
		   -M virt,virtualization=on \
		   -cpu cortex-a72 \
		   -m 512 \
		   -serial mon:stdio \
		   -device qemu-xhci \
		   -display none \
		   -net none
endif
ifeq ($(MACH),rpi3)
QEMU_OPTS+=-kernel $(O)/kernel.bin \
		   -initrd $(O)/initrd.img \
		   -M raspi3b \
		   -serial mon:stdio \
		   -display none \
		   -net none
endif
endif

ifneq ($(QEMU_SDCARD),)
QEMU_OPTS+=-drive if=sd,file=$(QEMU_SDCARD)
endif

ifeq ($(QEMU_DINT),1)
QEMU_OPTS+=-d int
endif
ifeq ($(QEMU_PAUSE),1)
QEMU_OPTS+=-S
endif

.PHONY: address error etc kernel src

all: kernel initrd

kernel:
	cd kernel && cargo build $(CARGO_BUILD_OPTS)
ifeq ($(ARCH),aarch64)
	$(LLVM_BASE)/llvm-strip -o $(O)/kernel.strip $(O)/kernel
	$(LLVM_BASE)/llvm-size $(O)/kernel.strip
	$(OBJCOPY) -O binary $(O)/kernel.strip $(O)/kernel.bin
endif
ifeq ($(MACH),orangepi3)
	$(MKIMAGE) \
		-A arm64 \
		-O linux \
		-T kernel \
		-C none \
		-a 0x48000000 \
		-e 0x48000000 \
		-n kernel \
		-d $(O)/kernel.bin \
		$(O)/uImage
endif

initrd:
	cd user && cargo build \
		--target=../etc/$(ARCH)-osdev5.json \
		-Z build-std=core,alloc,compiler_builtins \
		$(CARGO_COMMON_OPTS)
	mkdir -p $(O)/rootfs/bin $(O)/rootfs/sbin $(O)/rootfs/dev $(O)/rootfs/etc $(O)/rootfs/sys
	cp etc/initrd/passwd $(O)/rootfs/etc
	cp etc/initrd/shadow $(O)/rootfs/etc
	touch $(O)/rootfs/dev/.do_not_remove
	touch $(O)/rootfs/sys/.do_not_remove
	cp target/$(ARCH)-osdev5/$(PROFILE)/init $(O)/rootfs/init
	cp target/$(ARCH)-osdev5/$(PROFILE)/shell $(O)/rootfs/bin
	cp target/$(ARCH)-osdev5/$(PROFILE)/fuzzy $(O)/rootfs/bin
	cp target/$(ARCH)-osdev5/$(PROFILE)/ls $(O)/rootfs/bin
	cp target/$(ARCH)-osdev5/$(PROFILE)/cat $(O)/rootfs/bin
	cp target/$(ARCH)-osdev5/$(PROFILE)/hexd $(O)/rootfs/bin
	cp target/$(ARCH)-osdev5/$(PROFILE)/login $(O)/rootfs/sbin
	cd $(O)/rootfs && tar cf ../initrd.img `find -type f -printf "%P\n"`
ifeq ($(MACH),orangepi3)
	$(MKIMAGE) \
		-A arm64 \
		-O linux \
		-T ramdisk \
		-C none \
		-a 0x80000000 \
		-n initrd \
		-d $(O)/initrd.img \
		$(O)/uRamdisk
endif

test:
	cd fs/vfs && cargo test
	cd fs/memfs && cargo test
	cd fs/fat32 && cargo test

clean:
	cargo clean

doc:
	cd kernel && cargo doc --all-features --target=../etc/$(ARCH)-$(MACH).json

doc-open:
	cd kernel && cargo doc --open --all-features --target=../etc/$(ARCH)-$(MACH).json

clippy:
	cd kernel && cargo clippy $(CARGO_BUILD_OPTS)
	cd user && cargo clippy \
		--target=../etc/$(ARCH)-osdev5.json \
		-Zbuild-std=core,alloc,compiler_builtins $(CARGO_COMMON_OPTS)

qemu: all
	$(QEMU_PREFIX)qemu-system-$(ARCH) $(QEMU_OPTS)

gdb: all
	$(GDB) -x etc/gdbrc $(O)/kernel
