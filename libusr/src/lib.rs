#![feature(asm, alloc_error_handler)]
#![no_std]

use core::panic::PanicInfo;
use libsys::proc::ExitCode;

#[macro_use]
extern crate lazy_static;

pub mod file;
pub mod io;
pub mod os;
pub mod sys;
pub mod sync;

#[inline(never)]
extern "C" fn _signal_handler(arg: sys::Signal) -> ! {
    trace!("Entered signal handler: arg={:?}", arg);
    sys::sys_ex_sigreturn();
}

static mut SIGNAL_STACK: [u8; 4096] = [0; 4096];

#[link_section = ".text._start"]
#[no_mangle]
extern "C" fn _start(_arg: usize) -> ! {
    extern "Rust" {
        fn main() -> i32;
    }

    unsafe {
        sys::sys_ex_signal(
            _signal_handler as usize,
            SIGNAL_STACK.as_ptr() as usize + 4096,
        )
        .unwrap();
    }

    let res = unsafe { main() };
    sys::sys_exit(ExitCode::from(res));
}

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    // TODO print to stdout/stderr (if available)
    trace!("Panic ocurred: {}", pi);
    sys::sys_exit(ExitCode::from(-1));
}
