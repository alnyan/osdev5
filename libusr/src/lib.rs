#![feature(asm, alloc_error_handler)]
#![no_std]

use core::panic::PanicInfo;

pub mod io;
pub mod os;

pub mod sys {
    pub use libsys::signal::{Signal, SignalDestination};
    pub use libsys::termios;
    pub use libsys::calls::*;
    pub use libsys::stat::{self, STDERR_FILENO, STDIN_FILENO, STDOUT_FILENO};
}

#[inline(never)]
extern "C" fn _signal_handler(arg: sys::Signal) -> ! {
    trace!("Entered signal handler: arg={:?}", arg);
    unsafe {
        sys::sys_ex_sigreturn();
    }
}

static mut SIGNAL_STACK: [u8; 4096] = [0; 4096];

#[link_section = ".text._start"]
#[no_mangle]
extern "C" fn _start(_arg: usize) -> ! {
    extern "Rust" {
        fn main() -> i32;
    }
    unsafe {
        SIGNAL_STACK[0] = 1;
        sys::sys_ex_signal(_signal_handler as usize, SIGNAL_STACK.as_ptr() as usize + 4096);

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
