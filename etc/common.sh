if [ "$ARCH" = "" ]; then
    ARCH=aarch64
fi

case $ARCH in
    aarch64)
        if [ "$MACH" = "" ]; then
            MACH=qemu
        fi
        ;;
    x86_64)
        MACH=none
        ;;
esac

if [ "$PROFILE" = "" ]; then
    PROFILE=debug
fi

OUT_DIR=target/${ARCH}-${MACH}/${PROFILE}
