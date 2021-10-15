.section .text
.global __aa64_ctx_switch
.global __aa64_ctx_switch_to
.global __aa64_ctx_enter_kernel

.set PT_REGS_SIZE, 16 * 6

__aa64_ctx_enter_kernel:
    ldp x0, x1, [sp, #0]
    msr sp_el0, x0
    msr ttbr0_el1, x1

    mov x0, #4
    msr spsr_el1, x0
    ldp x0, x1, [sp, #16]
    msr elr_el1, x1
    add sp, sp, #32

    mov x1, xzr
    eret

__aa64_ctx_switch:
    sub sp, sp, #PT_REGS_SIZE

    stp x19, x20, [sp, #16 * 0]
    stp x21, x22, [sp, #16 * 1]
    stp x23, x24, [sp, #16 * 2]
    stp x25, x26, [sp, #16 * 3]
    stp x27, x28, [sp, #16 * 4]
    stp x29, x30, [sp, #16 * 5]

    mov x19, sp
    str x19, [x1]
__aa64_ctx_switch_to:
    ldr x0, [x0]
    mov sp, x0

    ldp x19, x20, [sp, #16 * 0]
    ldp x21, x22, [sp, #16 * 1]
    ldp x23, x24, [sp, #16 * 2]
    ldp x25, x26, [sp, #16 * 3]
    ldp x27, x28, [sp, #16 * 4]
    ldp x29, x30, [sp, #16 * 5]
    add sp, sp, #PT_REGS_SIZE

    ret
