// vi:ft=a64asm.asm:

.set SCTLR_EL2_RES1,        0x30C50830

.set SPSR_EL2_EL1h,         0x5
.set SPSR_EL2_MASK_DAIF,    0xF << 6
.set HCR_EL2_RW,            1 << 31
.set HCR_EL2_HCD,           1 << 29

.set CNTHCTL_EL2_EL1PCEN,   1 << 1
.set CNTHCTL_EL2_EL1PCTEN,  1 << 0

.macro ADR_REL reg, sym
    adrp \reg, \sym
    add \reg, \reg, #:lo12:\sym
.endm

.macro MOV_L reg, value
    mov \reg, #((\value) & 0xFFFF)
    movk \reg, #((\value) >> 16), lsl #16
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
    mrs x0, cnthctl_el2
    orr x0, x0, #(CNTHCTL_EL2_EL1PCTEN | CNTHCTL_EL2_EL1PCEN)
    msr cnthctl_el2, x0
    msr cntvoff_el2, xzr

    MOV_L x0, SCTLR_EL2_RES1
    msr sctlr_el2, x0

    mov x0, #HCR_EL2_RW
    msr hcr_el2, x0

    mov x0, #SPSR_EL2_EL1h
    orr x0, x0, #SPSR_EL2_MASK_DAIF
    msr spsr_el2, x0

    ADR_REL x0, 1f
    msr elr_el2, x0

    isb
    eret
1:
    dsb sy
    isb

    ADR_REL x0, bsp_stack_top
    mov sp, x0

    mov lr, xzr
    bl __aa64_bsp_main
    b .

.section .bss
.p2align 12
bsp_stack_bottom:
    .skip 32768
bsp_stack_top:
