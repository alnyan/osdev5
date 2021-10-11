// vi:ft=a64asm.asm:

.set SCTLR_EL2_RES1,        0x30C50830

.set SPSR_EL2_EL1h,         0x5
.set SPSR_EL2_MASK_DAIF,    0xF << 6
.set HCR_EL2_RW,            1 << 31
.set HCR_EL2_HCD,           1 << 29

.set CNTHCTL_EL2_EL1PCEN,   1 << 1
.set CNTHCTL_EL2_EL1PCTEN,  1 << 0

.section .text._entry
.global _entry
_entry:
    mov x8, x0

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

    adr x0, 1f
    msr elr_el2, x0

    isb
    eret
1:
    dsb sy
    isb

    // Zero .bss
    ADR_ABS x0, __bss_start_phys
    ADR_ABS x1, __bss_end_phys
1:
    cmp x0, x1
    beq 2f

    str xzr, [x0], #8

    b 1b
2:

    ADR_ABS x9, __aa64_entry_upper
    b __aa64_enter_upper

.section .text._entry_upper
__aa64_entry_upper:
    // x0 -- fdt address

    ADR_REL x1, bsp_stack_top
    mov sp, x1

    mov lr, xzr
    bl __aa64_bsp_main
    b .

.section .bss
.p2align 12
bsp_stack_bottom:
    .skip 32768
bsp_stack_top:

.section .data
.p2align 4
fdt_base_phys:
    .skip 8
