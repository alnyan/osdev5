.section .text
.global __aa64_ctx_switch
.global __aa64_ctx_switch_to
.global __aa64_ctx_enter_kernel

__aa64_ctx_enter_kernel:
    mov x0, #5
    msr spsr_el1, x0
    ldp x0, x1, [sp]
    msr elr_el1, x1
    add sp, sp, #16
    eret

__aa64_ctx_switch:
    sub sp, sp, #96

    stp x19, x20, [sp, #0]
    stp x21, x22, [sp, #16]
    stp x23, x24, [sp, #32]
    stp x25, x26, [sp, #48]
    stp x27, x29, [sp, #64]
    stp xzr, lr, [sp, #80]

    mov x19, sp
    str x19, [x1]
__aa64_ctx_switch_to:
    ldr x0, [x0]
    mov sp, x0

    ldp x19, x20, [sp, #0]
    ldp x21, x22, [sp, #16]
    ldp x23, x24, [sp, #32]
    ldp x25, x26, [sp, #48]
    ldp x27, x29, [sp, #64]
    ldp xzr, lr, [sp, #80]
    add sp, sp, #96

    ret
