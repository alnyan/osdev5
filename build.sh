#!/bin/sh

set -e

. etc/common.sh

CARGO_OPTS="--target ../etc/${ARCH}-${MACH}.json"
CARGO_FEATURES=""
LLVM_BIN=$(llvm-config --bindir)

if [ ! "$MACH" = "none" ]; then
    CARGO_FEATURES="${CARGO_FEATURES}mach_${MACH},"
fi

CARGO_OPTS="$CARGO_OPTS --features=$CARGO_FEATURES"

if [ "$PROFILE" = "release" ]; then
    CARGO_OPTS="$CARGO_OPTS --release"
fi

cd kernel
cargo build ${CARGO_OPTS}
cd ..

case $ARCH in
    aarch64)
        ${LLVM_BIN}/llvm-objcopy -O binary ${OUT_DIR}/kernel ${OUT_DIR}/kernel.bin
        ;;
    x86_64)
        mkdir -p ${OUT_DIR}/cdrom/boot/grub
        cp etc/x86_64-none.grub ${OUT_DIR}/cdrom/boot/grub/grub.cfg
        cp ${OUT_DIR}/kernel ${OUT_DIR}/cdrom/boot/kernel.elf
        grub-mkrescue -o ${OUT_DIR}/cdrom.iso ${OUT_DIR}/cdrom
        ;;
esac
