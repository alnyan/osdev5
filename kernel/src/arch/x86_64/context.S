.section .text
.global __x86_64_ctx_switch
.global __x86_64_ctx_switch_to
.global __x86_64_ctx_enter_kernel
.global __x86_64_ctx_enter_from_fork

__x86_64_ctx_enter_user:
    pop %rcx
    pop %rdi
    pop %rdi
    pop %rdx

    push %rdx

    push $0x1B
    push %rcx

    push $0x200

    push $0x23
    push %rdx

    iretq

__x86_64_ctx_enter_kernel:
    pop %rdi
    pop %rdx

    // 8-byte fixup for proper stack alignment
    push %rdx

    mov %rsp, %rcx

    push $0x10
    push %rcx

    push $0x200

    push $0x08
    push %rdx

    iretq

__x86_64_ctx_enter_from_fork:
    pop %rdi
    pop %rsi
    pop %rdx
    pop %r10
    pop %r8
    pop %r9

    pop %rax
    xor %rax, %rax

    pop %r11 // rsp3
    pop %rcx // rip3

    push $0x1B
    push %r11

    push $0x200

    push $0x23
    push %rcx

    iretq

__x86_64_ctx_switch:
    // %rsi -- src ctx ptr
    // %rdi -- dst ctx ptr

    // push %tss_rsp0
    // TODO save gs_base
    mov (4 + TSS)(%rip), %rax
    push %rax

    // push %cr3
    mov %cr3, %rax
    push %rax

    push %r15
    push %r14
    push %r13
    push %r12
    push %rbx
    push %rbp

    // TODO SAVE FP CONTEXT

    mov %rsp, (%rsi)
__x86_64_ctx_switch_to:
    mov (%rdi), %rsp

    // TODO RESTORE FP CONTEXT

    pop %rbp
    pop %rbx
    pop %r12
    pop %r13
    pop %r14
    pop %r15

    // pop %cr3
    pop %rax
    test %rax, %rax
    jz 1f
    mov %rax, %cr3
1:
    // TODO set gs_base = rax

    // pop %tss_rsp0
    pop %rax
    mov %rax, (4 + TSS)(%rip)

    ret
