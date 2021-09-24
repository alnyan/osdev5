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

case $1 in
    ""|build)
        CARGO_CMD=build
        ;;
    clean)
        CARGO_CMD=clean
        ;;
    clippy)
        CARGO_CMD=clippy
        ;;
    doc)
        shift
        if [ x$1 = xopen ]; then
            CARGO_OPTS="$CARGO_OPTS --open"
        fi
        CARGO_CMD=doc
        ;;
esac

cd kernel
cargo ${CARGO_CMD} ${CARGO_OPTS}
cd ..

if [ ${CARGO_CMD} = "build" ]; then
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
fi
