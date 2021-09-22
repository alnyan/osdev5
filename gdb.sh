#!/bin/sh

. etc/common.sh

gdb-multiarch -x etc/gdbrc target/${ARCH}-${MACH}/${PROFILE}/kernel
