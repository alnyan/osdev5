// vi:ft=a64asm.asm:
.section .text._entry
.global _entry
_entry:
    // Preserve FDT base address
    mov x8, x0

    mrs x1, mpidr_el1
    and x1, x1, #3
    beq 2f
1:
    wfe
    b 1b

2:
    mrs x0, currentel
    cmp x0, #8
    bne 1f

    b .

1:

    // TODO zero .bss

    ADR_ABS x9, __aa64_entry_upper
    b __aa64_enter_upper

.section .text._entry_upper
__aa64_entry_upper:
    // x0 -- FDT base address

    ADR_REL x1, bsp_stack_top
    mov sp, x1

    mov lr, xzr
    bl __aa64_bsp_main

    b .

.section .bss
.p2align 4
bsp_stack_bottom:
    .skip 16384
bsp_stack_top:
