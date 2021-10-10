ENTRY(_entry);

KERNEL_OFFSET = 0xFFFFFF8000000000;
BASE_OFFSET = 0x40080000;

SECTIONS {
    . = BASE_OFFSET;

    .text.lower : {
        *(.text._entry)
    }

    . = ALIGN(16);
    . = . + KERNEL_OFFSET;

    PROVIDE(__kernel_start = .);

    .text : AT(. - KERNEL_OFFSET) {
        *(.text._entry_upper)
        *(.text*)
    }

    . = ALIGN(4K);
    .rodata : AT(. - KERNEL_OFFSET) {
        *(.rodata*)
    }

    . = ALIGN(4K);
    .data : AT(. - KERNEL_OFFSET) {
        *(.data*)
    }

    . = ALIGN(4K);
    .tables : AT(. - KERNEL_OFFSET) {
        *(.tables)
    }

    . = ALIGN(4K);
    .bss : AT(. - KERNEL_OFFSET) {
        PROVIDE(__bss_start = .);
        *(COMMON)
        *(.bss*)
        . = ALIGN(4K);
        PROVIDE(__bss_end = .);
    }

    PROVIDE(__kernel_end = .);
}
