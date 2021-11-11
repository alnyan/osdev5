#![feature(asm, alloc_error_handler)]
#![no_std]

use core::panic::PanicInfo;

pub mod io;
pub mod os;

pub mod sys {
    pub use syscall::calls::*;
    pub use syscall::stat::{self, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};
    pub use syscall::termios;
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
