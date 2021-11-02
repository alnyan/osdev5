#![feature(asm, alloc_error_handler)]
#![no_std]

use core::panic::PanicInfo;

pub mod mem;
pub mod os;
pub mod io;

pub mod sys {
    pub use syscall::*;
}

#[link_section = ".text._start"]
#[no_mangle]
extern "C" fn _start(_arg: usize) -> ! {
    extern "Rust" {
        fn main() -> i32;
    }
    unsafe {
        sys::sys_exit(main());
    }
}

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    // TODO formatted messages
    trace!("Panic ocurred: {}", pi);
    unsafe {
        sys::sys_exit(-1);
    }
}
