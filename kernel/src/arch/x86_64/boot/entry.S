.section .text._multiboot
.set MAGIC,     0xE85250D6
.set ARCH,      0x0
.set HDRLEN,    16
.set CHKSUM,    (-(MAGIC + ARCH + HDRLEN)) & 0xFFFFFFFF

.long MAGIC
.long ARCH
.long HDRLEN
.long CHKSUM

.short 5
.short 0
.long 20
.long 800
.long 600
.long 32

.short 0
.long 8

.section .text._entry
.global _entry
_entry:
.code32
    cli
    lea (multiboot_registers - KERNEL_OFFSET), %edi
    mov %eax, 0(%edi)
    mov %ebx, 4(%edi)

    // Setup paging tables
    lea (_entry_upper - KERNEL_OFFSET), %ebx
    jmp __x86_64_enter_upper

.code64
_entry_upper:
    lea bsp_stack_top(%rip), %rax
    mov %rax, %rsp

    mov multiboot_registers(%rip), %edi
    mov (4 + multiboot_registers)(%rip), %esi
    call __x86_64_bsp_main

.section .bss
    .align 16
bsp_stack_bottom:
    .skip 65536
bsp_stack_top:
multiboot_registers:
    .skip 8
