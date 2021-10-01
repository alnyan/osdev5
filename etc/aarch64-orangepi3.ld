ENTRY(_entry);

MEMORY {
    ram : ORIGIN = 0x48000000, LENGTH = 992M
}

SECTIONS {
    PROVIDE(__kernel_start = .);

    .text : {
        *(.text._entry)
        *(.text*)
    } >ram

    . = ALIGN(4K);
    .rodata : {
        *(.rodata*)
    } >ram

    . = ALIGN(4K);
    .data : {
        *(.data*)
    } >ram

    . = ALIGN(4K);
    .bss : {
        PROVIDE(__bss_start = .);
        *(COMMON)
        *(.bss*)
        . = ALIGN(4K);
        PROVIDE(__bss_end = .);
    } >ram

    PROVIDE(__kernel_end = .);
}
