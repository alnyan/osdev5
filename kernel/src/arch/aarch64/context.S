.section .text
.global __aa64_ctx_switch
.global __aa64_ctx_switch_to
.global __aa64_ctx_enter_kernel
.global __aa64_ctx_enter_from_fork

.set PT_REGS_SIZE, 16 * 7

__aa64_ctx_enter_user:
    ldp x0, x1, [sp, #0]
    msr sp_el0, x0

    msr spsr_el1, xzr
    ldp x0, x1, [sp, #16]
    msr elr_el1, x1
    add sp, sp, #32

    mov x1, xzr
__return_to_user:
    eret

__aa64_ctx_enter_kernel:
    msr sp_el0, xzr

    mov x0, #5
    msr spsr_el1, x0
    ldp x0, x1, [sp, #0]
    msr elr_el1, x1
    add sp, sp, #16

    mov x1, xzr
    eret

__aa64_ctx_enter_from_fork:
    ldp x0,  x1,  [sp, #16 * 0]
    msr sp_el0, x0
    msr elr_el1, x1
    msr spsr_el1, xzr

    ldp x1,  x2,  [sp, #16 * 1]
    ldp x3,  x4,  [sp, #16 * 2]
    ldp x5,  x6,  [sp, #16 * 3]
    ldp x7,  x8,  [sp, #16 * 4]
    ldp x9,  x10, [sp, #16 * 5]
    ldp x11, x12, [sp, #16 * 6]
    ldp x13, x14, [sp, #16 * 7]
    ldp x15, x16, [sp, #16 * 8]
    ldp x17, x18, [sp, #16 * 9]

    mov x0, xzr

    eret

__aa64_ctx_switch:
    sub sp, sp, #PT_REGS_SIZE

    stp x19, x20, [sp, #16 * 0]
    stp x21, x22, [sp, #16 * 1]
    stp x23, x24, [sp, #16 * 2]
    stp x25, x26, [sp, #16 * 3]
    stp x27, x28, [sp, #16 * 4]
    stp x29, x30, [sp, #16 * 5]
    mrs x19, TTBR0_EL1
    mrs x20, TPIDR_EL0
    stp x19, x20, [sp, #16 * 6]

    mov x19, sp
    str x19, [x1]
__aa64_ctx_switch_to:
    ldr x0, [x0]
    mov sp, x0

    ldp x19, x20, [sp, #16 * 6]
    msr TTBR0_EL1, x19
    msr TPIDR_EL0, x20
    ldp x19, x20, [sp, #16 * 0]
    ldp x21, x22, [sp, #16 * 1]
    ldp x23, x24, [sp, #16 * 2]
    ldp x25, x26, [sp, #16 * 3]
    ldp x27, x28, [sp, #16 * 4]
    ldp x29, x30, [sp, #16 * 5]
    add sp, sp, #PT_REGS_SIZE

    ret
