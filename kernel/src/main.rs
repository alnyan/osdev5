#![feature(
    global_asm,
    const_for,
    const_mut_refs,
    const_raw_ptr_deref,
    const_fn_fn_ptr_basics
)]
#![no_std]
#![no_main]

#[macro_use]
extern crate cfg_if;

#[macro_use]
pub mod debug;
pub mod arch;
pub mod dev;
pub mod mem;
pub mod sync;

#[panic_handler]
fn panic_handler(_pi: &core::panic::PanicInfo) -> ! {
    loop {}
}
