//! Process and thread manipulation facilities

use crate::init;
use crate::sync::IrqSafeSpinLock;
use crate::mem;
use alloc::{
    boxed::Box,
    collections::{BTreeMap},
};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::arch::platform::cpu::{self, Cpu};

pub mod elf;
pub mod process;
pub use process::{Pid, Process, ProcessRef, State as ProcessState};
pub mod io;
pub use io::ProcessIo;

pub mod wait;

pub mod sched;
pub use sched::Scheduler;
//pub(self) use sched::SCHED;

// <<<<<<< HEAD
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

///// Performs a task switch.
/////
///// See [Scheduler::switch]
//pub fn switch() {
//    SCHED.switch(false);
//}

///
pub fn process(id: Pid) -> ProcessRef {
    PROCESSES.lock().get(&id).unwrap().clone()
}

macro_rules! spawn {
    (fn ($dst_arg:ident : usize) $body:block, $src_arg:expr) => {{
        #[inline(never)]
        extern "C" fn __inner_func($dst_arg : usize) -> ! {
            let __res = $body;
            {
                todo!();
                // #![allow(unreachable_code)]
                // SCHED.current_process().exit(__res);
                panic!();
            }
        }

        let __proc = $crate::proc::Process::new_kernel(__inner_func, $src_arg).unwrap();
        $crate::proc::sched::enqueue(__proc.id());
    }};

    (fn () $body:block) => (spawn!(fn (_arg: usize) $body, 0usize))
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
pub unsafe fn enter(is_bsp: bool) -> ! {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    let sched = Cpu::get().scheduler();
    sched.init();

    COUNTER.fetch_add(1, Ordering::Release);
    while COUNTER.load(Ordering::Acquire) != cpu::count() {
        cortex_a::asm::nop();
    }

    if is_bsp {
        sched::enqueue(Process::new_kernel(init::init_fn, 0).unwrap().id());
    } else {
        spawn!(fn () {
            loop {}
        });
    }
    // if let Some((start, end)) = initrd {
    //     let initrd = Box::into_raw(Box::new((mem::virtualize(start), mem::virtualize(end))));

    //     spawn!(fn (initrd_ptr: usize) {
    //         loop {}
    //         // debugln!("Running kernel init process");

    //         // let (start, _end) = unsafe { *(initrd_ptr as *const (usize, usize)) };
    //         // Process::execve(|space| elf::load_elf(space, start as *const u8), 0).unwrap();
    //         // panic!("This code should not run");
    //     }, initrd as usize);
    // }

    sched.enter();
}
