//! osdve5 crate (lol)
#![feature(
    asm,
    global_asm,
    const_for,
    const_mut_refs,
    const_raw_ptr_deref,
    const_fn_fn_ptr_basics,
    const_fn_trait_bound,
    const_trait_impl,
    const_panic,
    panic_info_message,
    alloc_error_handler,
    linked_list_cursors,
    const_btree_new,
    maybe_uninit_uninit_array
)]
#![no_std]
#![no_main]
#![warn(missing_docs)]

#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate bitflags;
extern crate alloc;

#[macro_use]
pub mod debug;

pub mod arch;
pub mod config;
pub mod dev;
pub mod fs;
pub mod init;
pub mod mem;
pub mod proc;
pub mod sync;
pub mod syscall;
pub mod util;

#[panic_handler]
fn panic_handler(pi: &core::panic::PanicInfo) -> ! {
    unsafe {
        asm!("msr daifset, #2");
        use crate::arch::platform::cpu::{self, Cpu};

        crate::arch::platform::smp::send_ipi(true, (1 << cpu::count()) - 1, 0);
    }

    use cortex_a::registers::MPIDR_EL1;
    use tock_registers::interfaces::Readable;

    errorln!("Panic on node{}: {:?}", MPIDR_EL1.get() & 0xF, pi);
    // TODO
    loop {
        unsafe {
            asm!("wfe");
        }
    }
}
