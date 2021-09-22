#![feature(global_asm)]

#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic_handler(_pi: &PanicInfo) -> ! {
    loop {}
}

global_asm!(r#"
.section .text._entry
.global _entry
_entry:
    mrs x1, mpidr_el1
    and x1, x1, #3
    beq 2f
1:
    wfe
    b 1b

2:
    b .
"#);
