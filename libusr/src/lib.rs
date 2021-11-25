#![feature(asm, alloc_error_handler)]
#![no_std]

#[macro_use]
extern crate lazy_static;

extern crate alloc;

use core::panic::PanicInfo;
use libsys::{debug::TraceLevel, ProgramArgs, proc::ExitCode};

mod allocator;
pub mod env;
pub mod file;
pub mod io;
pub mod os;
pub mod sys;
pub mod sync;
pub mod thread;
pub mod signal;

#[link_section = ".text._start"]
#[no_mangle]
extern "C" fn _start(arg: &'static ProgramArgs) -> ! {
    extern "Rust" {
        fn main() -> i32;
    }

    unsafe {
        thread::init_main();
        env::setup_env(arg);
    }

    let res = unsafe { main() };
    sys::sys_exit(ExitCode::from(res));
}

#[panic_handler]
fn panic_handler(pi: &PanicInfo) -> ! {
    // TODO unwind to send panic argument back to parent thread
    // TODO print to stdout/stderr (if available)
    let thread = thread::current();
    trace!(TraceLevel::Error, "{:?} panicked: {:?}", thread, pi);
    sys::sys_exit(ExitCode::from(-1));
}
