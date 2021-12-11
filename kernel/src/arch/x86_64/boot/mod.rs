use crate::arch::x86_64::{gdt, idt};
use core::arch::global_asm;

#[no_mangle]
extern "C" fn __x86_64_bsp_main(mb_checksum: u32, mb_info_ptr: u32) -> ! {
    // TODO enable FP support for kernel/user
    // Setup a proper GDT
    unsafe {
        gdt::init();
        idt::init(|_| {});
    }

    loop {}
}

global_asm!(include_str!("macros.S"), options(att_syntax));
global_asm!(include_str!("entry.S"), options(att_syntax));
global_asm!(include_str!("upper.S"), options(att_syntax));
