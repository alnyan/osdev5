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

CARGO_BUILD_OPTS=--target=../etc/$(ARCH)-$(MACH).json
ifneq ($(MACH),)
CARGO_BUILD_OPTS+=--features mach_$(MACH)
endif
ifeq ($(PROFILE),release)
CARGO_BUILD_OPTS+=--release
endif

QEMU_OPTS=-s \
		  -chardev stdio,id=serial1,mux=on
ifeq ($(ARCH),x86_64)
$(error TODO)
else
ifeq ($(MACH),qemu)
QEMU_OPTS+=-kernel $(O)/kernel.bin \
		   -initrd $(O)/initrd.img \
		   -M virt,virtualization=on \
		   -cpu cortex-a72 \
		   -m 512 \
		   -serial chardev:serial1 \
		   -device qemu-xhci \
		   -display none \
		   -net none
endif
ifeq ($(MACH),rpi3)
QEMU_OPTS+=-kernel $(O)/kernel.bin \
		   -dtb etc/bcm2837-rpi-3-b-plus.dtb \
		   -M raspi3b \
		   -serial null \
		   -serial chardev:serial1
endif
endif

ifeq ($(QEMU_DINT),1)
QEMU_OPTS+=-d int
endif
ifeq ($(QEMU_PAUSE),1)
QEMU_OPTS+=-S
endif

.PHONY: address error etc kernel src

all: kernel

kernel:
	cd kernel && cargo build $(CARGO_BUILD_OPTS)
	cd init && cargo build --target=../etc/$(ARCH)-osdev5.json -Z build-std=core,alloc,compiler_builtins
	echo "This is a test file" >$(O)/test.txt
	cd $(O) && tar cf initrd.img test.txt
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

test:
	cd fs/vfs && cargo test
	cd fs/memfs && cargo test

clean:
	cargo clean

doc:
	cd kernel && cargo doc --all-features --target=../etc/$(ARCH)-$(MACH).json

doc-open:
	cd kernel && cargo doc --open --all-features --target=../etc/$(ARCH)-$(MACH).json

clippy:
	cd kernel && cargo clippy $(CARGO_BUILD_OPTS)

qemu: all
	$(QEMU_PREFIX)qemu-system-$(ARCH) $(QEMU_OPTS)

gdb: all
	$(GDB) -x etc/gdbrc $(O)/kernel
