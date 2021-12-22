.code32

.section .text._entry
__x86_64_enter_upper:
    mov $(PTE_PRESENT | PTE_WRITABLE | PTE_USERSPACE), %edx
    // Setup PML4
    lea (KERNEL_FIXED - KERNEL_OFFSET), %edi
    lea (KERNEL_FIXED + 4096 - KERNEL_OFFSET), %esi
    mov %edx, %eax
    or %esi, %eax
    //  pml4[0] = %eax
    mov %eax, (%edi)
    //  pml4[511] = %eax
    mov %eax, 4088(%edi)

    // Setup PDPT
    mov %esi, %edi
    lea (KERNEL_FIXED + 8192 - KERNEL_OFFSET), %esi
    xor %ecx, %ecx
1:
    // %eax = &table[%ecx] | attrs
    mov %esi, %eax
    or %edx, %eax
    mov %eax, (%edi, %ecx, 8)

    add $4096, %esi
    inc %ecx
    cmp $16, %ecx
    jne 1b

    // Setup PDs
    lea (KERNEL_FIXED + 8192 - KERNEL_OFFSET), %edi
    mov $(PTE_PRESENT | PTE_BLOCK | PTE_WRITABLE), %edx
    mov $(512 * 16), %ecx
1:
    dec %ecx

    // %eax = attrs | (i << 21)
    mov %ecx, %eax
    shl $21, %eax
    or %edx, %eax

    mov %eax, (%edi, %ecx, 8)

    test %ecx, %ecx
    jnz 1b

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
    lea (KERNEL_FIXED - KERNEL_OFFSET), %edi
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
