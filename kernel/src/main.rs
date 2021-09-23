#![feature(global_asm)]

#![no_std]
#![no_main]

#[macro_use]
extern crate cfg_if;

#[macro_use]
pub mod debug;
pub mod arch;
pub mod mem;

#[panic_handler]
fn panic_handler(_pi: &core::panic::PanicInfo) -> ! {
    loop {}
}
