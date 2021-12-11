.macro isr_nerr, n
exc_isr_\n:
    cli
    pushq $0
    pushq $\n
    jmp __x86_64_isr_common
.endm

// ISR for exception with an error code
.macro isr_yerr, n
exc_isr_\n:
    cli
    pushq $\n
    jmp __x86_64_isr_common
.endm

.section .text
__x86_64_isr_common:
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
    call __x86_64_exception_handler

1:
    cli
    hlt
    jmp 1b

isr_nerr 0
isr_nerr 1
isr_nerr 2
isr_nerr 3
isr_nerr 4
isr_nerr 5
isr_nerr 6
isr_nerr 7
isr_yerr 8
isr_nerr 9
isr_yerr 10
isr_yerr 11
isr_yerr 12
isr_yerr 13
isr_yerr 14
isr_nerr 15
isr_nerr 16
isr_yerr 17
isr_nerr 18
isr_nerr 19
isr_nerr 20
isr_nerr 21
isr_nerr 22
isr_nerr 23
isr_nerr 24
isr_nerr 25
isr_nerr 26
isr_nerr 27
isr_nerr 28
isr_nerr 29
isr_yerr 30
isr_nerr 31

.section .rodata
.global __x86_64_exception_vectors
__x86_64_exception_vectors:
    .quad exc_isr_0
    .quad exc_isr_1
    .quad exc_isr_2
    .quad exc_isr_3
    .quad exc_isr_4
    .quad exc_isr_5
    .quad exc_isr_6
    .quad exc_isr_7
    .quad exc_isr_8
    .quad exc_isr_9
    .quad exc_isr_10
    .quad exc_isr_11
    .quad exc_isr_12
    .quad exc_isr_13
    .quad exc_isr_14
    .quad exc_isr_15
    .quad exc_isr_16
    .quad exc_isr_17
    .quad exc_isr_18
    .quad exc_isr_19
    .quad exc_isr_20
    .quad exc_isr_21
    .quad exc_isr_22
    .quad exc_isr_23
    .quad exc_isr_24
    .quad exc_isr_25
    .quad exc_isr_26
    .quad exc_isr_27
    .quad exc_isr_28
    .quad exc_isr_29
    .quad exc_isr_30
    .quad exc_isr_31
