//! Process and thread manipulation facilities

use crate::sync::IrqSafeNullLock;
use crate::util::InitOnce;
use alloc::{rc::Rc, collections::{BTreeMap, VecDeque}};
use error::Errno;

pub mod elf;
pub mod process;
pub use process::{Process, ProcessRef, Pid, State as ProcessState};

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
    const USTACK_VIRT_TOP: usize = 0x100000000;

    fn new() -> Self {
        let mut this = Self {
            queue: VecDeque::new(),
            idle: None,
            current: None,
        };

        this.idle = Some(unsafe { (*Process::new_kernel(idle_fn, 0).unwrap().get()).id() });

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
            let proc = PROCESSES.lock().get(&id).unwrap().clone();
            (*proc.get()).state = ProcessState::Running;
            proc
        };

        (*thread.get()).enter();
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
                    lock.get(&next).unwrap().clone()
                )
            };

            if !discard {
                unsafe {
                    assert_eq!((*from.get()).state, ProcessState::Running);
                    (*from.get()).state = ProcessState::Ready;
                }
            }
            unsafe {
                assert_eq!((*to.get()).state, ProcessState::Ready);
                (*to.get()).state = ProcessState::Running;
            }

            (from, to)
        };

        if !Rc::ptr_eq(&from, &to) {
            // FIXME This is ugly
            unsafe {
                (*from.get()).switch_to(to.get());
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
    loop {}
}

macro_rules! spawn {
    (fn ($dst_arg:ident : usize) $body:block, $src_arg:expr) => {{
        #[inline(never)]
        extern "C" fn __inner_func($dst_arg : usize) -> ! {
            $body;
            #[allow(unreachable_code)]
            unsafe { (*SCHED.current_process().get()).exit() }
        }

        let __proc = $crate::proc::Process::new_kernel(__inner_func, $src_arg).unwrap();
        $crate::proc::SCHED.enqueue(unsafe { (*__proc.get()).id() });
    }};

    (fn () $body:block) => (spawn!(fn (_arg: usize) $body, 0usize))
}

/// Performs a task switch.
///
/// See [Scheduler::switch]
pub fn switch() {
    SCHED.switch(false);
}

///
pub static SCHED: Scheduler = Scheduler {
    inner: InitOnce::new(),
};

///
pub static PROCESSES: IrqSafeNullLock<BTreeMap<Pid, ProcessRef>> = IrqSafeNullLock::new(BTreeMap::new());

/// Sets up initial process and enters it.
///
/// See [Scheduler::enter]
///
/// # Safety
///
/// Unsafe: May only be called once.
pub unsafe fn enter(initrd: Option<(usize, usize)>) -> ! {
    SCHED.init();
    spawn!(fn () {
        debugln!("Henlo");
    });
    // if let Some((start, end)) = initrd {
    //     let initrd = Box::into_raw(Box::new((mem::virtualize(start), mem::virtualize(end))));

    //     spawn!(fn (initrd_ptr: usize) {
    //         debugln!("Running kernel init process");

    //         let (start, _end) = unsafe { *(initrd_ptr as *const (usize, usize)) };
    //         let proc = unsafe { &mut *SCHED.current_process().get() };
    //         proc.execve(|space| elf::load_elf(space, start as *const u8), 0)
    //             .unwrap();
    //         panic!("This code should not run");
    //     }, initrd as usize);
    // }
    SCHED.enter();
}
