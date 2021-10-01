// vi:ft=a64asm.asm:

.macro ADR_REL reg, sym
    adrp \reg, \sym
    add \reg, \reg, #:lo12:\sym
.endm

.section .text._entry
.global _entry
_entry:
    mrs x1, mpidr_el1
    and x1, x1, #3
    beq 2f
1:
    wfe
    b 1b

2:

    // Zero .bss
    ADR_REL x0, __bss_start
    ADR_REL x1, __bss_end
1:
    cmp x0, x1
    beq 2f
    stp xzr, xzr, [x0], #16
    b 1b
2:

    ADR_REL x0, bsp_stack_top
    mov sp, x0

    b __aa64_bsp_main

.section .bss
.p2align 4
bsp_stack_bottom:
    .skip 16384
bsp_stack_top:
