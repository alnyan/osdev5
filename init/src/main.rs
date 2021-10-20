#![feature(asm)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[link_section = ".text._start"]
#[no_mangle]
extern "C" fn _start(arg: usize) -> ! {
    let mut c0 = arg;
    let mut c1: usize;
    loop {
        unsafe {
            asm!("mrs {}, cntpct_el0", out(reg) c1);
            asm!("svc #0", inout("x0") c0, in("x1") c1);
        }

        for _ in 0..1000000 {
            unsafe { asm!("nop"); }
        }
    }
}

#[panic_handler]
fn panic_handler(_pi: &PanicInfo) -> ! {
    loop {}
}
