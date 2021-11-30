//!
use crate::proc::{Thread, ThreadRef, THREADS};
use crate::util::InitOnce;
use alloc::{collections::VecDeque, rc::Rc};
use crate::sync::{IrqSafeSpinLock, IrqSafeSpinLockGuard};
use crate::arch::platform::cpu::{self, Cpu};
use cortex_a::registers::{MPIDR_EL1, DAIF};
use core::ops::Deref;
use tock_registers::interfaces::Readable;

struct SchedulerInner {
    queue: VecDeque<u32>,
    idle: Option<u32>,
    current: Option<u32>,
}

/// Process scheduler state and queues
pub struct Scheduler {
    inner: InitOnce<IrqSafeSpinLock<SchedulerInner>>,
}

impl SchedulerInner {
    fn new() -> Self {
        let mut this = Self {
            queue: VecDeque::new(),
            idle: None,
            current: None,
        };

        this.idle = Some(Thread::new_kernel(None, idle_fn, 0).unwrap().id());

        this
    }
}

impl Scheduler {
    ///
    pub const fn new() -> Self {
        Self {
            inner: InitOnce::new()
        }
    }

    ///
    pub fn queue_size(&self) -> usize {
        let lock = self.inner.get().lock();
        let c = if lock.current.is_some() {
            1
        } else {
            0
        };
        lock.queue.len() + c
    }

    /// Initializes inner data structure:
    ///
    /// * idle thread
    /// * process list/queue data structs
    pub fn init(&self) {
        self.inner.init(IrqSafeSpinLock::new(SchedulerInner::new()));
    }

    /// Schedules a thread for execution
    pub fn enqueue(&self, tid: u32) {
        self.inner.get().lock().queue.push_back(tid);
    }

    /// Removes given `tid` from execution queue
    pub fn dequeue(&self, tid: u32) {
        self.inner.get().lock().queue.retain(|&p| p != tid)
    }

    /// Performs initial process entry.
    ///
    /// # Safety
    ///
    /// Unsafe: may only be called once, repeated calls will cause UB.
    pub unsafe fn enter(&self) -> ! {
        let thread = {
            let mut inner = self.inner.get().lock();
            let id = if inner.queue.is_empty() {
                inner.idle.unwrap()
            } else {
                inner.queue.pop_front().unwrap()
            };

            inner.current = Some(id);
            THREADS.lock().get(&id).unwrap().clone()
        };

        asm!("msr daifset, #2");
        Thread::enter((MPIDR_EL1.get() & 0xF) as u32, thread)
    }

    /// This hack is required to be called from execve() when downgrading current
    /// process from kernel to user.
    ///
    /// # Safety
    ///
    /// Unsafe: only allowed to be called from Process::execve()
    pub unsafe fn hack_current_tid(&self, old: u32, new: u32) {
        let mut lock = self.inner.get().lock();
        match lock.current {
            Some(t) if t == old => {
                lock.current = Some(new);
            }
            _ => {}
        }
    }

    /// Switches to the next task scheduled for execution. If there're
    /// none present in the queue, switches to the idle task.
    pub fn switch(&self, discard: bool, sched_lock: IrqSafeSpinLockGuard<()>) {
        let (from, to) = {
            let mut inner = self.inner.get().lock();
            let current = inner.current.unwrap();

//<<<<<<< HEAD
            if !discard && current != inner.idle.unwrap() {
//=======
//            if !discard && current != 0 {
//>>>>>>> feat/thread
                // Put the process into the back of the queue
                if !enqueue_somewhere_else((MPIDR_EL1.get() & 0xF) as usize, current, &sched_lock) {
                    inner.queue.push_back(current);
                }
            }

            let next = if inner.queue.is_empty() {
                inner.idle.unwrap()
            } else {
                inner.queue.pop_front().unwrap()
            };

            inner.current = Some(next);
            let (from, to) = {
                let lock = THREADS.lock();
                (
                    lock.get(&current).unwrap().clone(),
                    lock.get(&next).unwrap().clone(),
                )
            };

            (from, to)
        };

        if !Rc::ptr_eq(&from, &to) {
            unsafe {
//<<<<<<< HEAD
                drop(sched_lock);
//                Process::switch((MPIDR_EL1.get() & 0xF) as u32, from, to, discard);
//=======
                asm!("msr daifset, #2");
                Thread::switch((MPIDR_EL1.get() & 0xF) as u32, from, to, discard);
//>>>>>>> feat/thread
            }
        }
    }

    /// Returns a [Rc]-reference to currently running Thread
    pub fn current_thread(&self) -> ThreadRef {
        let inner = self.inner.get().lock();
        let id = inner.current.unwrap();
        THREADS.lock().get(&id).unwrap().clone()
    }

    // /// Returns a Rc-reference to currently running process
    // pub fn current_process(&self) -> ProcessRef {
    //     let inner = self.inner.get().lock();
    //     let current = inner.current.unwrap();
    //     PROCESSES.lock().get(&current).unwrap().clone()
    // }
}

// <<<<<<< HEAD
// // pub fn is_ready() -> bool {
// //     SCHED.inner.is_initialized()
// // }
// =======
// /// Returns `true` if the scheduler has been initialized
// pub fn is_ready() -> bool {
//     SCHED.inner.is_initialized()
// }
// >>>>>>> feat/thread

#[inline(never)]
extern "C" fn idle_fn(_a: usize) -> ! {
    loop {
        cortex_a::asm::wfi();
    }
}

pub fn current_thread() -> ThreadRef {
    let guard = SCHED_LOCK.lock();
    unsafe { Cpu::get().scheduler().current_thread() }
}

/// Performs a task switch.
///
/// See [Scheduler::switch]
pub fn switch(discard: bool) {
    assert!(DAIF.matches_all(DAIF::I::SET));
    let guard = SCHED_LOCK.lock();
    unsafe { Cpu::get().scheduler().switch(discard, guard); }
}

///
pub fn enqueue_to(cpu: usize, tid: u32) {
    todo!()
    //let _lock = SCHED_LOCK.lock();
    //debugln!("Queue {} to cpu{}", pid, cpu);
    //unsafe {
    //    cpu::by_index(cpu).scheduler().enqueue(pid)
    //}
}

///
pub fn enqueue(tid: u32) {
    let _lock = SCHED_LOCK.lock();
    let mut min_idx = 0;
    let mut min_cnt = usize::MAX;
    for (i, cpu) in unsafe { cpu::cpus() }.enumerate() {
        let size = cpu.scheduler().queue_size();
        if size < min_cnt {
            min_cnt = size;
            min_idx = i;
        }
    }

    // debugln!("Queue {} to cpu{}", pid, min_idx);
    unsafe {
        cpu::by_index(min_idx).scheduler().enqueue(tid)
    }
}

///
pub fn enqueue_somewhere_else(ignore: usize, tid: u32, _guard: &IrqSafeSpinLockGuard<()>) -> bool {
    let mut min_idx = 0;
    //let mut min_cnt = usize::MAX;
    static mut LAST: usize = 0;
    //for (i, cpu) in unsafe { cpu::cpus() }.enumerate() {
    //for (i, cpu) in wacky_cpu_iterate() {
    //    if i == ignore {
    //        continue;
    //    }
    //    let size = cpu.scheduler().queue_size();
    //    if size < min_cnt {
    //        min_cnt = size;
    //        min_idx = i;
    //    }
    //}
    unsafe {
        LAST = (LAST + 1) % cpu::count();
        min_idx = LAST;
    }

    if min_idx == ignore {
        false
    } else {
        unsafe {
            cpu::by_index(min_idx).scheduler().enqueue(tid)
        }
        true
    }
}

///
pub fn dequeue(tid: u32) {
    // TODO process can be rescheduled to other CPU between scheduler locks
    let lock = SCHED_LOCK.lock();
    let cpu_id = Thread::get(tid).unwrap().cpu();
    unsafe {
        cpu::by_index(cpu_id as usize).scheduler().dequeue(tid);
    }
}

static SCHED_LOCK: IrqSafeSpinLock<()> = IrqSafeSpinLock::new(());

// TODO maybe move this into a per-CPU struct
// /// Global scheduler struct
// pub static SCHED: Scheduler = Scheduler {
//     inner: InitOnce::new(),
// };
