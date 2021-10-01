// vi:ft=a64asm.asm:

.set UART0_BASE,    0x05000000

.set SPSR_EL2_EL1h, 0x5
.set HCR_EL2_RW,    1 << 31
.set HCR_EL2_HCD,   1 << 29

.macro ADR_REL reg, sym
    adrp \reg, \sym
    add \reg, \reg, #:lo12:\sym
.endm

.section .text._entry
.global _entry
_entry:
    // Test for EL2
    mrs x0, CurrentEL
    lsr x0, x0, #2
    cmp x0, #2
    bne 1f
    // Exit EL2
    // TODO cnthtctl_el2 setup

    ADR_REL x0, 1f
    msr elr_el2, x0
    mov x0, #SPSR_EL2_EL1h
    msr spsr_el2, x0
    mov x0, #(HCR_EL2_RW | HCR_EL2_HCD)
    msr hcr_el2, x0

    eret
1:
    dsb sy
    isb

    ADR_REL x0, bsp_stack_top
    mov sp, x0

    b __aa64_bsp_main

.section .bss
.p2align 12
bsp_stack_bottom:
    .skip 32768
bsp_stack_top:
