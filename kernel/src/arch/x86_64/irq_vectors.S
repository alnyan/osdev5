.macro irq_entry no
__x86_64_irq_\no:
    cli
    pushq $0
    pushq $\no

    push %rax
    push %rcx
    push %rdx
    push %rbx
    push %rbp
    push %rsi
    push %rdi

    push %r8
    push %r9
    push %r10
    push %r11
    push %r12
    push %r13
    push %r14
    push %r15

    mov %rsp, %rdi
    call __x86_64_irq_handler

    jmp .
.endm

.section .text

__x86_64_irq_0:
    cli

    push %rax
    push %rcx
    push %rdx
    push %rbx
    push %rbp
    push %rsi
    push %rdi

    push %r8
    push %r9
    push %r10
    push %r11
    push %r12
    push %r13
    push %r14
    push %r15

    mov $0x3F8, %dx
    mov $'T', %al
    outb %al, %dx

    mov $0x20, %al
    mov $0x20, %dx
    outb %al, %dx

    call sched_yield

    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %r11
    pop %r10
    pop %r9
    pop %r8

    pop %rdi
    pop %rsi
    pop %rbp
    pop %rbx
    pop %rdx
    pop %rcx
    pop %rax

    iretq

irq_entry 1
irq_entry 2
irq_entry 3
irq_entry 4
irq_entry 5
irq_entry 6
irq_entry 7
irq_entry 8
irq_entry 9
irq_entry 10
irq_entry 11
irq_entry 12
irq_entry 13
irq_entry 14
irq_entry 15

.section .rodata
.global __x86_64_irq_vectors
__x86_64_irq_vectors:
    .quad __x86_64_irq_0
    .quad __x86_64_irq_1
    .quad __x86_64_irq_2
    .quad __x86_64_irq_3
    .quad __x86_64_irq_4
    .quad __x86_64_irq_5
    .quad __x86_64_irq_6
    .quad __x86_64_irq_7
    .quad __x86_64_irq_8
    .quad __x86_64_irq_9
    .quad __x86_64_irq_10
    .quad __x86_64_irq_11
    .quad __x86_64_irq_12
    .quad __x86_64_irq_13
    .quad __x86_64_irq_14
    .quad __x86_64_irq_15
