#![feature(asm)]
#![no_std]
#![no_main]

use core::panic::PanicInfo;

static RODATA: [u8; 4] = [1, 2, 3, 4];
static mut WDATA: [u8; 4] = [1, 2, 3, 4];
static mut WBSS: [u8; 16] = [0; 16];

#[link_section = ".text._start"]
#[no_mangle]
extern "C" fn _start(_arg: usize) -> ! {
    let mut c0;

    unsafe {
        let d: &mut [u8; 4] = &mut *(&WBSS as *const _ as *mut _);
        d[0] = 2;
    }

    c0 = unsafe { &mut WDATA as *mut _ as usize };
    c0 = unsafe { &mut WBSS as *mut _ as usize };
    let mut c1 = 1u64;
    loop {
        unsafe {
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
