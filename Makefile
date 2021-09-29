ARCH?=aarch64
ifeq ($(ARCH),aarch64)
MACH?=qemu
endif
GDB?=gdb-multiarch

LLVM_BASE=$(shell llvm-config --bindir)
OBJCOPY=$(LLVM_BASE)/llvm-objcopy

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
		   -serial chardev:serial0
endif
endif

.PHONY: address error etc kernel src

all: kernel

kernel:
	cd kernel && cargo build $(CARGO_BUILD_OPTS)
ifeq ($(ARCH),aarch64)
	$(OBJCOPY) -O binary $(O)/kernel $(O)/kernel.bin
endif

clean:
	cargo clean

qemu: all
	qemu-system-$(ARCH) $(QEMU_OPTS)

gdb: all
	$(GDB) -x etc/gdbrc $(O)/kernel
