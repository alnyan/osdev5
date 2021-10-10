ARCH?=aarch64
ifeq ($(ARCH),aarch64)
MACH?=qemu
endif
GDB?=gdb-multiarch

LLVM_BASE=$(shell llvm-config --bindir)
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
		  -chardev stdio,id=serial0,mux=on
ifeq ($(ARCH),x86_64)
$(error TODO)
else
ifeq ($(MACH),qemu)
QEMU_OPTS+=-kernel $(O)/kernel.bin \
		   -M virt,virtualization=off \
		   -cpu cortex-a72 \
		   -m 512 \
		   -serial chardev:serial0 \
		   -device virtio-serial-pci
endif
ifeq ($(MACH),rpi3b)
QEMU_OPTS+=-kernel $(O)/kernel.bin \
		   -M raspi3b \
		   -serial null \
		   -serial chardev:serial0
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
ifeq ($(ARCH),aarch64)
	$(OBJCOPY) -O binary $(O)/kernel $(O)/kernel.bin
endif
ifeq ($(MACH),orangepi3)
	$(LLVM_BASE)/llvm-strip $(O)/kernel
	$(LLVM_BASE)/llvm-size $(O)/kernel
endif

clean:
	cargo clean

clippy:
	cd kernel && cargo clippy $(CARGO_BUILD_OPTS)

qemu: all
	$(QEMU_PREFIX)qemu-system-$(ARCH) $(QEMU_OPTS)

gdb: all
	$(GDB) -x etc/gdbrc $(O)/kernel
