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
    // stack.push(frame.x[18]);
    // stack.push(frame.x[17]);
    // stack.push(frame.x[16]);
    // stack.push(frame.x[15]);
    // stack.push(frame.x[14]);
    // stack.push(frame.x[13]);
    // stack.push(frame.x[12]);
    // stack.push(frame.x[11]);
    // stack.push(frame.x[10]);
    // stack.push(frame.x[9]);
    // stack.push(frame.x[8]);
    // stack.push(frame.x[7]);
    // stack.push(frame.x[6]);
    // stack.push(frame.x[5]);
    // stack.push(frame.x[4]);
    // stack.push(frame.x[3]);
    // stack.push(frame.x[2]);
    // stack.push(frame.x[1]);

    // stack.push(frame.elr_el1 as usize);
    // stack.push(frame.sp_el0 as usize);
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
    stp x19, xzr, [sp, #16 * 6]

    mov x19, sp
    str x19, [x1]
__aa64_ctx_switch_to:
    ldr x0, [x0]
    mov sp, x0

    ldp x19, xzr, [sp, #16 * 6]
    msr TTBR0_EL1, x19
    ldp x19, x20, [sp, #16 * 0]
    ldp x21, x22, [sp, #16 * 1]
    ldp x23, x24, [sp, #16 * 2]
    ldp x25, x26, [sp, #16 * 3]
    ldp x27, x28, [sp, #16 * 4]
    ldp x29, x30, [sp, #16 * 5]
    add sp, sp, #PT_REGS_SIZE

    ret
