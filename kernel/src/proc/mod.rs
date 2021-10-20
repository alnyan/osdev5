//! Process and thread manipulation facilities

use crate::mem;
use crate::sync::IrqSafeNullLock;
use crate::util::InitOnce;
use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

pub mod elf;
pub mod process;
pub use process::{Pid, Process, ProcessRef, State as ProcessState};

struct SchedulerInner {
    queue: VecDeque<Pid>,
    idle: Option<Pid>,
    current: Option<Pid>,
}

/// Process scheduler state and queues
pub struct Scheduler {
    inner: InitOnce<IrqSafeNullLock<SchedulerInner>>,
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
        self.inner.init(IrqSafeNullLock::new(SchedulerInner::new()));
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

        asm!("msr daifclr, #2");
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
                asm!("msr daifclr, #2");
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

// TODO maybe move this into a per-CPU struct
/// Global scheduler struct
pub static SCHED: Scheduler = Scheduler {
    inner: InitOnce::new(),
};

/// Global list of all processes in the system
pub static PROCESSES: IrqSafeNullLock<BTreeMap<Pid, ProcessRef>> =
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

        spawn!(fn () {
            debugln!("Terminator process started");

            for _ in 0..2000000 {
                cortex_a::asm::nop();
            }

            let pid = Pid::user(1);
            debugln!("Killing {}", pid);
            PROCESSES.lock().get(&pid).unwrap().exit(123);
            let status = Process::waitpid(pid).unwrap();
            debugln!("{} exit status was {:?}", pid, status);

            1
        });
    }
    SCHED.enter();
}
