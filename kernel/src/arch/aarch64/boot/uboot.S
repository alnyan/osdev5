// vi:ft=a64asm.asm:

.macro MOV_L reg, value
    mov \reg, #((\value) & 0xFFFF)
    movk \reg, #((\value) >> 16), lsl #16
.endm

.macro ADR_REL reg, sym
    adrp \reg, \sym
    add \reg, \reg, #:lo12:\sym
.endm

.macro ADR_ABS reg, sym
    movz \reg, #:abs_g3:\sym
    movk \reg, #:abs_g2_nc:\sym
    movk \reg, #:abs_g1_nc:\sym
    movk \reg, #:abs_g0_nc:\sym
.endm

.set PTE_BLOCK_AF,      1 << 10
.set PTE_BLOCK_ISH,     3 << 8
.set PTE_PRESENT,       1 << 0

.set MAIR_EL1_Attr0_Normal_Inner_NC,    (4 << 0)
.set MAIR_EL1_Attr0_Normal_Outer_NC,    (4 << 4)
.set MAIR_EL1_Attr1_Device,             (0 << 12)
.set MAIR_EL1_Attr1_Device_nGnRE,       (1 << 8)

.set ID_AA64MMFR0_EL1_TGran4,           (0xF << 28)

.set TCR_EL1_IPS_SHIFT,                 32

.set TCR_EL1_TG1_4K,                    (2 << 30)
.set TCR_EL1_SH1_Outer,                 (2 << 28)
.set TCR_EL1_ORGN1_NC,                  (0 << 26)
.set TCR_EL1_IRGN1_NC,                  (0 << 24)
.set TCR_EL1_T1SZ_SHIFT,                16

.set TCR_EL1_TG0_4K,                    (0 << 14)
.set TCR_EL1_SH0_Outer,                 (2 << 12)
.set TCR_EL1_ORGN0_NC,                  (0 << 10)
.set TCR_EL1_IRGN0_NC,                  (0 << 8)
.set TCR_EL1_T0SZ_SHIFT,                0

.set TCR_EL1_ATTRS,                     (TCR_EL1_TG1_4K | TCR_EL1_SH1_Outer | TCR_EL1_TG0_4K | TCR_EL1_SH0_Outer | (25 << TCR_EL1_T1SZ_SHIFT) | (25 << TCR_EL1_T0SZ_SHIFT))

.set SCTLR_EL1_I,                       (1 << 12)
.set SCTLR_EL1_C,                       (1 << 2)
.set SCTLR_EL1_M,                       (1 << 0)


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

.global __aa64_enter_upper
.type __aa64_enter_upper, %function
__aa64_enter_upper:
    // x8 -- FDT base
    // x9 -- upper entry point

    // Setup TTBR1_EL1
    // TODO fix macros
    ADR_ABS x5, KERNEL_TTBR1
    ADR_ABS x6, KERNEL_OFFSET

    // x5 = KERNEL_TTBR1 physical address
    sub x5, x5, x6

    // Fill KERNEL_TTBR1 table with upper-mapped Normal memory
.fill_ttbr1:
    mov x2, #256
1:
    sub x2, x2, #1

    // x0 = (x2 << 30) | attrs...
    lsl x1, x2, #30
    mov x0, #(PTE_BLOCK_ISH | PTE_BLOCK_AF | PTE_PRESENT)
    orr x0, x0, x1

    str x0, [x5, x2, lsl #3]

    cbnz x2, 1b

.init_mmu_regs:
    mov x0, #(MAIR_EL1_Attr0_Normal_Outer_NC | MAIR_EL1_Attr0_Normal_Inner_NC | MAIR_EL1_Attr1_Device | MAIR_EL1_Attr1_Device_nGnRE)
    msr mair_el1, x0

    // Test for 4KiB page support
    mrs x0, ID_AA64MMFR0_EL1
    mov x1, ID_AA64MMFR0_EL1_TGran4
    tst x0, x1
    bne .no_4k_gran

    // x0 = PARange
    and x0, x0, #0xF
    lsl x0, x0, #TCR_EL1_IPS_SHIFT
    MOV_L x1, TCR_EL1_ATTRS
    orr x0, x0, x1
    msr tcr_el1, x0

    msr ttbr0_el1, x5
    msr ttbr1_el1, x5

    dsb ish
    isb

    mrs x0, sctlr_el1
    orr x0, x0, #SCTLR_EL1_M
    msr sctlr_el1, x0

    mov x0, x8
    br x9
.no_4k_gran:
    b .
.size __aa64_enter_upper, . - __aa64_enter_upper

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
