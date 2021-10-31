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
    panic_info_message,
    alloc_error_handler,
    linked_list_cursors,
    const_btree_new
)]
#![no_std]
#![no_main]
#![deny(missing_docs)]

#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate bitflags;
extern crate alloc;

#[macro_use]
pub mod debug;

pub mod arch;
pub mod dev;
pub mod fs;
pub mod mem;
pub mod proc;
pub mod sync;
#[allow(missing_docs)]
pub mod syscall;
pub mod util;

#[panic_handler]
fn panic_handler(pi: &core::panic::PanicInfo) -> ! {
    unsafe {
        asm!("msr daifset, #2");
    }

    errorln!("Panic: {:?}", pi);
    // TODO
    loop {}
}
