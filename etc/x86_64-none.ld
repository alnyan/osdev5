ENTRY(_entry);

KERNEL_OFFSET = 0xFFFFFF8000000000;

SECTIONS {
    . = 0x400000 + KERNEL_OFFSET;

    .text : AT(. - KERNEL_OFFSET) {
        KEEP(*(.text._multiboot))
        *(.text._entry)
        *(.text*)
    }

    .rodata : AT(. - KERNEL_OFFSET) {
        *(.rodata*)
    }

    .data : AT(. - KERNEL_OFFSET) {
        *(.data*)
    }

    .bss : AT(. - KERNEL_OFFSET) {
        *(COMMON)
        *(.bss*)
        . = ALIGN(4K);
    }

    PROVIDE(__kernel_end = .);
}
