//! osdve5 crate (lol)
#![feature(
    const_for,
    const_mut_refs,
    const_fn_fn_ptr_basics,
    const_fn_trait_bound,
    const_trait_impl,
    const_btree_new,
    panic_info_message,
    alloc_error_handler,
    asm_const,
    core_intrinsics,
    const_generics_defaults,
)]
#![no_std]
#![no_main]
#![warn(missing_docs)]

#[macro_use]
extern crate kernel_macros;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate bitflags;
extern crate alloc;

#[macro_use]
pub mod debug;
//
pub mod arch;
pub mod config;
pub mod dev;
pub mod fs;
pub mod font;
pub mod init;
pub mod mem;
pub mod proc;
pub mod sync;
// pub mod syscall;
pub mod util;

#[panic_handler]
fn panic_handler(pi: &core::panic::PanicInfo) -> ! {
    unsafe {
        arch::intrin::irq_disable();
    }

    errorln!("Panic: {:?}", pi);
    // TODO
    loop {}
}
