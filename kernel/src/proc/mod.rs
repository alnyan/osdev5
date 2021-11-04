//! Process and thread manipulation facilities

use crate::init;
use crate::sync::IrqSafeSpinLock;
use alloc::collections::BTreeMap;

pub mod elf;
pub mod process;
pub use process::{Pid, Process, ProcessRef, State as ProcessState};

pub mod wait;

pub mod sched;
pub use sched::Scheduler;
pub(self) use sched::SCHED;

// macro_rules! spawn {
//     (fn ($dst_arg:ident : usize) $body:block, $src_arg:expr) => {{
//         #[inline(never)]
//         extern "C" fn __inner_func($dst_arg : usize) -> ! {
//             let __res = $body;
//             {
//                 #![allow(unreachable_code)]
//                 SCHED.current_process().exit(__res);
//                 panic!();
//             }
//         }
//
//         let __proc = $crate::proc::Process::new_kernel(__inner_func, $src_arg).unwrap();
//         $crate::proc::SCHED.enqueue(__proc.id());
//     }};
//
//     (fn () $body:block) => (spawn!(fn (_arg: usize) $body, 0usize))
// }

/// Performs a task switch.
///
/// See [Scheduler::switch]
pub fn switch() {
    SCHED.switch(false);
}

///
pub fn process(id: Pid) -> ProcessRef {
    PROCESSES.lock().get(&id).unwrap().clone()
}

/// Global list of all processes in the system
pub(self) static PROCESSES: IrqSafeSpinLock<BTreeMap<Pid, ProcessRef>> =
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
    SCHED.enqueue(Process::new_kernel(init::init_fn, 0).unwrap().id());
    SCHED.enter();
}
