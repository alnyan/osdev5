//!
use crate::proc::{Pid, Process, ProcessRef, PROCESSES};
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use alloc::{collections::VecDeque, rc::Rc};

struct SchedulerInner {
    queue: VecDeque<Pid>,
    idle: Option<Pid>,
    current: Option<Pid>,
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

        this.idle = Some(Process::new_kernel(idle_fn, 0).unwrap().id());

        this
    }
}

impl Scheduler {
    /// Initializes inner data structure:
    ///
    /// * idle thread
    /// * process list/queue data structs
    pub fn init(&self) {
        self.inner.init(IrqSafeSpinLock::new(SchedulerInner::new()));
    }

    /// Schedules a thread for execution
    pub fn enqueue(&self, pid: Pid) {
        self.inner.get().lock().queue.push_back(pid);
    }

    ///
    pub fn dequeue(&self, pid: Pid) {
        self.inner.get().lock().queue.retain(|&p| p != pid)
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
            PROCESSES.lock().get(&id).unwrap().clone()
        };

        asm!("msr daifset, #2");
        Process::enter(thread)
    }

    /// This hack is required to be called from execve() when downgrading current
    /// process from kernel to user.
    ///
    /// # Safety
    ///
    /// Unsafe: only allowed to be called from Process::execve()
    pub unsafe fn hack_current_pid(&self, new: Pid) {
        self.inner.get().lock().current = Some(new);
    }

    /// Switches to the next task scheduled for execution. If there're
    /// none present in the queue, switches to the idle task.
    pub fn switch(&self, discard: bool) {
        let (from, to) = {
            let mut inner = self.inner.get().lock();
            let current = inner.current.unwrap();

            if !discard && current != Pid::IDLE {
                // Put the process into the back of the queue
                inner.queue.push_back(current);
            }

            let next = if inner.queue.is_empty() {
                inner.idle.unwrap()
            } else {
                inner.queue.pop_front().unwrap()
            };

            inner.current = Some(next);
            let (from, to) = {
                let lock = PROCESSES.lock();
                (
                    lock.get(&current).unwrap().clone(),
                    lock.get(&next).unwrap().clone(),
                )
            };

            (from, to)
        };

        if !Rc::ptr_eq(&from, &to) {
            unsafe {
                asm!("msr daifset, #2");
                Process::switch(from, to, discard);
            }
        }
    }

    /// Returns a Rc-reference to currently running process
    pub fn current_process(&self) -> ProcessRef {
        let inner = self.inner.get().lock();
        let current = inner.current.unwrap();
        PROCESSES.lock().get(&current).unwrap().clone()
    }
}

#[inline(never)]
extern "C" fn idle_fn(_a: usize) -> ! {
    loop {
        cortex_a::asm::wfi();
    }
}

// TODO maybe move this into a per-CPU struct
/// Global scheduler struct
pub static SCHED: Scheduler = Scheduler {
    inner: InitOnce::new(),
};
