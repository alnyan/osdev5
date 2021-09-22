#!/bin/sh

set -e

. etc/common.sh

./build.sh

QEMU_OPTS="-chardev id=char0,mux=on,backend=stdio \
           -s"

case $ARCH in
    aarch64)
        case $MACH in
            qemu)
                QEMU_OPTS="$QEMU_OPTS \
                           -M virt,virtualization=on \
                           -m 512 \
                           -serial chardev:char0 \
                           -cpu cortex-a72 \
                           -kernel target/${ARCH}-${MACH}/${PROFILE}/kernel.bin"
                ;;
        esac
        ;;
    x86_64)
        QEMU_OPTS="$QEMU_OPTS \
                   -M q35 \
                   -m 512 \
                   -serial chardev:char0 \
                   -cpu host \
                   -enable-kvm \
                   -cdrom target/${ARCH}-${MACH}/${PROFILE}/cdrom.iso"
        ;;
esac

qemu-system-${ARCH} ${QEMU_OPTS} $@
