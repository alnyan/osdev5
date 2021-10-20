//! Process and thread manipulation facilities

use crate::mem;
use crate::sync::IrqSafeNullLock;
use alloc::{
    boxed::Box,
    collections::{BTreeMap},
};

pub mod elf;
pub mod process;
pub use process::{Pid, Process, ProcessRef, State as ProcessState};

pub mod sched;
pub use sched::Scheduler;
pub(self) use sched::SCHED;

macro_rules! spawn {
    (fn ($dst_arg:ident : usize) $body:block, $src_arg:expr) => {{
        #[inline(never)]
        extern "C" fn __inner_func($dst_arg : usize) -> ! {
            let __res = $body;
            {
                #![allow(unreachable_code)]
                SCHED.current_process().exit(__res);
                panic!();
            }
        }

        let __proc = $crate::proc::Process::new_kernel(__inner_func, $src_arg).unwrap();
        $crate::proc::SCHED.enqueue(__proc.id());
    }};

    (fn () $body:block) => (spawn!(fn (_arg: usize) $body, 0usize))
}

/// Performs a task switch.
///
/// See [Scheduler::switch]
pub fn switch() {
    SCHED.switch(false);
}

/// Global list of all processes in the system
pub(self) static PROCESSES: IrqSafeNullLock<BTreeMap<Pid, ProcessRef>> =
    IrqSafeNullLock::new(BTreeMap::new());

/// Sets up initial process and enters it.
///
/// See [Scheduler::enter]
///
/// # Safety
///
/// Unsafe: May only be called once.
pub unsafe fn enter(initrd: Option<(usize, usize)>) -> ! {
    SCHED.init();
    if let Some((start, end)) = initrd {
        let initrd = Box::into_raw(Box::new((mem::virtualize(start), mem::virtualize(end))));

        spawn!(fn (initrd_ptr: usize) {
            debugln!("Running kernel init process");

            let (start, _end) = unsafe { *(initrd_ptr as *const (usize, usize)) };
            Process::execve(|space| elf::load_elf(space, start as *const u8), 0).unwrap();
            panic!("This code should not run");
        }, initrd as usize);
    }
    SCHED.enter();
}
