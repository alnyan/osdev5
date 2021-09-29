//! osdve5 crate (lol)
#![feature(
    asm,
    global_asm,
    const_for,
    const_mut_refs,
    const_raw_ptr_deref,
    const_fn_fn_ptr_basics,
    panic_info_message
)]
#![no_std]
#![no_main]
#![deny(missing_docs)]

#[macro_use]
extern crate cfg_if;

#[macro_use]
pub mod debug;
pub mod arch;
pub mod dev;
pub mod mem;
pub mod sync;

#[panic_handler]
fn panic_handler(pi: &core::panic::PanicInfo) -> ! {
    if let Some(msg) = pi.message() {
        debugln!("Panic occurred: {:?}", msg);
    } else {
        debugln!("Panic occurred");
    }
    debugln!("Panic location: {:?}", pi.location());
    loop {}
}
