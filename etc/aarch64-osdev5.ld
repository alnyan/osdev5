ENTRY(_start);

PHDRS {
    text PT_LOAD ;
    rodata PT_LOAD ;
    data PT_LOAD ;
}

SECTIONS {
    . = 0x400000;

    .text : {
        *(.text._start)
        *(.text*)
        *(.eh_frame*)
    } :text

    . = ALIGN(0x1000);
    .rodata : {
        *(.rodata*)
    } :rodata

    . = ALIGN(0x1000);
    .data : {
        *(.data*)
    } :data

    .bss : {
        *(COMMON)
        *(.bss*)
    } :data
}
