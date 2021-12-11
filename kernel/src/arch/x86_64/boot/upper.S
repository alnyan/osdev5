.code32

.section .text._entry
__x86_64_enter_upper:
    // Setup paging table
    mov $(PTE_PRESENT | PTE_BLOCK | PTE_WRITABLE | PTE_USERSPACE), %eax
    // Fill PD0: 0..1GiB
    lea (KERNEL_PD0 - KERNEL_OFFSET), %edi
    mov $512, %ecx
1:
    dec %ecx

    mov %ecx, %edx
    shl $21, %edx
    or %eax, %edx

    mov %edx, (%edi, %ecx, 8)

    test %ecx, %ecx
    jnz 1b

    // Fill PD1: 1GiB..2GiB
    lea (KERNEL_PD1 - KERNEL_OFFSET), %edi
    mov $512, %ecx
1:
    dec %ecx

    mov %ecx, %edx
    add $512, %edx
    shl $21, %edx
    or %eax, %edx

    mov %edx, (%edi, %ecx, 8)

    test %ecx, %ecx
    jnz 1b

    // Setup PDPT
    mov $(PTE_PRESENT | PTE_WRITABLE | PTE_USERSPACE), %eax
    lea (KERNEL_PDPT - KERNEL_OFFSET), %edi

    lea (KERNEL_PD0 - KERNEL_OFFSET), %esi
    or %eax, %esi
    mov %esi, (%edi)

    lea (KERNEL_PD1 - KERNEL_OFFSET), %esi
    or %eax, %esi
    mov %esi, 8(%edi)

    // Setup PML4
    lea (KERNEL_PML4 - KERNEL_OFFSET), %edi
    lea (KERNEL_PDPT - KERNEL_OFFSET), %esi
    or %eax, %esi
    mov %esi, (%edi)
    mov %esi, 4088(%edi)

    // Enable PAE/PSE
    mov %cr4, %eax
    or $((1 << 5) | (1 << 4)), %eax
    mov %eax, %cr4

    // Enable EFER.LME
    mov $0xC0000080, %ecx
    rdmsr
    or $(1 << 8), %eax
    wrmsr

    // Set CR3
    mov %edi, %cr3

    // Enable paging
    mov %cr0, %eax
    or $(1 << 31), %eax
    mov %eax, %cr0

    lgdt (gdtr64 - KERNEL_OFFSET)
    ljmp $0x08, $(1f - KERNEL_OFFSET)
1:
    mov $0x10, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

.code64
    movabsq $KERNEL_OFFSET, %rax
    add %rax, %rbx
    jmp *%rbx

.section .rodata
.code32
.align 16
gdt64:
    .quad 0
    .quad 0x00209A0000000000
    .quad 0x0000920000000000
gdt_end64:
.align 16
gdtr64:
    .short gdt_end64 - gdt64 - 1
    .long gdt64 - KERNEL_OFFSET
