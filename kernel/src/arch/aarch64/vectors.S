.section .text
.global aa64_el1_vectors
.p2align 12
aa64_el1_vectors:
.el1_sp_el0_sync:
    b .
.p2align 7
.el1_sp_el0_irq:
    b .
.p2align 7
.el1_sp_el0_fiq:
    b .
.p2align 7
.el1_sp_el0_serror:
    b .

.p2align 7
.el1_sp_el1_sync:
    sub sp, sp, #192

    stp x0, x1, [sp, #0]
    stp x2, x3, [sp, #16]
    stp x4, x5, [sp, #32]
    stp x6, x7, [sp, #48]
    stp x8, x9, [sp, #64]
    stp x10, x11, [sp, #80]
    stp x12, x13, [sp, #96]
    stp x14, x15, [sp, #112]
    stp x16, x17, [sp, #128]
    stp x18, x29, [sp, #144]

    mrs x0, elr_el1
    stp x30, x0, [sp, #160]

    mrs x0, esr_el1
    mrs x1, far_el1
    stp x0, x1, [sp, #176]

    mov x0, sp
    bl __aa64_exc_handler

    b .
.p2align 7
.el1_sp_el1_irq:
    b .
.p2align 7
.el1_sp_el1_fiq:
    b .
.p2align 7
.el1_sp_el1_serror:
    b .

.p2align 7
.el0_aa64_sync:
    b .
.p2align 7
.el0_aa64_irq:
    b .
.p2align 7
.el0_aa64_fiq:
    b .
.p2align 7
.el0_aa64_serror:
    b .

.p2align 7
.el0_aa32_sync:
    b .
.p2align 7
.el0_aa32_irq:
    b .
.p2align 7
.el0_aa32_fiq:
    b .
.p2align 7
.el0_aa32_serror:
    b .
