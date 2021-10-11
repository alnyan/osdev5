//! osdve5 crate (lol)
#![feature(
    asm,
    global_asm,
    const_for,
    const_mut_refs,
    const_raw_ptr_deref,
    const_fn_fn_ptr_basics,
    const_fn_trait_bound,
    const_panic,
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
pub mod util;

use core::fmt::{Write, self};
struct DummyUart;

impl Write for DummyUart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            unsafe {
                core::ptr::write_volatile(0xFFFFFFC000000000 as *mut u32, b as u32);
                for _ in 0..100000 {
                    asm!("nop");
                }
            }
        }
        Ok(())
    }
}

#[panic_handler]
fn panic_handler(pi: &core::panic::PanicInfo) -> ! {
    let mut u = DummyUart;
    write!(&mut u, "Panic occurred!\r\n");
    write!(&mut u, "{:?}", pi);
    // TODO
    loop {}
}
