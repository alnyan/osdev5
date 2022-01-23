.section .text
.global __x86_64_syscall_entry
__x86_64_syscall_entry:
    // Syscalls only happen from user space, so
    // relying on TSS.RSP0 is safe here I guess
    mov %rsp, scratch(%rip)
    mov (4 + TSS)(%rip), %rsp

    // Now on kernel stack
    // Push the whole state
    push %rcx   // saved %rip
    push %r11   // saved %rflags
    mov scratch(%rip), %r11
    push %r11   // saved %rsp

    push %r12
    push %r13
    push %r14
    push %r15
    push %rbp
    push %rbx

    push %rax

    push %r9
    push %r8
    push %r10
    push %rdx
    push %rsi
    push %rdi

    mov %rsp, %rdi
    call __x86_64_syscall

    pop %rdi
    pop %rsi
    pop %rdx
    pop %r10
    pop %r8
    pop %r9

    pop %rax

    pop %rbx
    pop %rbp
    pop %r15
    pop %r14
    pop %r13
    pop %r12

    pop %rdi
    pop %r11
    pop %rcx
    mov %rdi, %rsp

    sysretq

.section .bss
scratch:
    .skip 8
