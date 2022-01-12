//! Process and thread manipulation facilities

use crate::init;
use crate::sync::IrqSafeSpinLock;
use alloc::collections::BTreeMap;
use libsys::proc::{Pid, Tid};

pub mod elf;
pub mod thread;
pub(self) use thread::Context;
pub use thread::{State as ThreadState, Thread, ThreadRef};
pub mod process;
pub use process::{Process, ProcessRef, ProcessState};
pub mod io;
pub use io::ProcessIo;

pub mod wait;

pub mod sched;
pub use sched::Scheduler;
pub(self) use sched::SCHED;

/// Performs a task switch.
///
/// See [Scheduler::switch]
pub fn switch() {
    SCHED.switch(false);
}

#[no_mangle]
extern "C" fn sched_yield() {
    SCHED.switch(false);
}

pub(self) static PROCESSES: IrqSafeSpinLock<BTreeMap<Pid, ProcessRef>> =
    IrqSafeSpinLock::new(BTreeMap::new());

pub(self) static THREADS: IrqSafeSpinLock<BTreeMap<Tid, ThreadRef>> =
    IrqSafeSpinLock::new(BTreeMap::new());

/// Sets up initial process and enters it.
///
/// See [Scheduler::enter]
///
/// # Safety
///
/// Unsafe: May only be called once.
pub unsafe fn enter() -> ! {
    SCHED.init();
    Process::new_kernel(init::init_fn, 0).unwrap().enqueue();
    SCHED.enter();
}
