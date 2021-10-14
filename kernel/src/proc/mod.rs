#![allow(missing_docs)]

use crate::sync::IrqSafeNullLock;
use crate::util::InitOnce;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::rc::Rc;
use core::cell::{RefCell, UnsafeCell};
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicU32, Ordering};

pub use crate::arch::platform::context::{self, Context};

pub type ProcessRef = Rc<UnsafeCell<Process>>;

pub struct Process {
    ctx: Context,
    id: u32,
}

struct SchedulerInner {
    processes: BTreeMap<u32, ProcessRef>,
    queue: VecDeque<u32>,
    idle: u32,
    current: Option<u32>,
}

pub struct Scheduler {
    inner: InitOnce<IrqSafeNullLock<SchedulerInner>>,
}

impl SchedulerInner {
    fn new_kernel(&mut self, entry: extern "C" fn(usize) -> !, arg: usize) -> u32 {
        static LAST_PID: AtomicU32 = AtomicU32::new(0);

        let id = LAST_PID.fetch_add(1, Ordering::Relaxed);
        let proc = Process {
            ctx: Context::kernel(entry as usize, arg),
            id,
        };
        debugln!("Created kernel process with PID {}", id);

        assert!(self
            .processes
            .insert(id, Rc::new(UnsafeCell::new(proc)))
            .is_none());

        id
    }

    fn new() -> Self {
        let mut this = Self {
            processes: BTreeMap::new(),
            queue: VecDeque::new(),
            idle: 0,
            current: None,
        };

        this.idle = this.new_kernel(idle_fn, 0);

        this
    }
}

impl Scheduler {
    pub fn new_kernel(&self, entry: extern "C" fn(usize) -> !, arg: usize) -> u32 {
        self.inner.get().lock().new_kernel(entry, arg)
    }

    pub fn init(&self) {
        self.inner.init(IrqSafeNullLock::new(SchedulerInner::new()));
    }

    pub fn enqueue(&self, pid: u32) {
        self.inner.get().lock().queue.push_back(pid);
    }

    pub fn current(&self) -> Option<ProcessRef> {
        let mut inner = self.inner.get().lock();
        inner
            .current
            .as_ref()
            .and_then(|id| inner.processes.get(id))
            .map(|r| r.clone())
    }

    pub unsafe fn enter(&self) -> ! {
        let thread = {
            let mut inner = self.inner.get().lock();
            let id = if inner.queue.is_empty() {
                inner.idle
            } else {
                inner.queue.pop_front().unwrap()
            };

            inner.current = Some(id);
            inner.processes.get(&id).unwrap().clone()
        };

        (*thread.get()).ctx.enter();
    }

    pub fn switch(&self) {
        let (from, to) = {
            let mut inner = self.inner.get().lock();
            let current = inner.current.unwrap();
            // Put the process into the back of the queue
            inner.queue.push_back(current);
            let next = if inner.queue.is_empty() {
                inner.idle
            } else {
                inner.queue.pop_front().unwrap()
            };

            inner.current = Some(next);
            (
                inner.processes.get(&current).unwrap().clone(),
                inner.processes.get(&next).unwrap().clone()
            )
        };

        if !Rc::ptr_eq(&from, &to) {
            // FIXME This is ugly
            unsafe {
                (*from.get()).ctx.switch(&mut (*to.get()).ctx);
            }
        }
    }
}

extern "C" fn idle_fn(_a: usize) -> ! {
    loop {}
}

extern "C" fn f0(a: usize) -> ! {
    debugln!("Thread #{} started", a);
    loop {
        debug!("{}", a);
    }
}

pub fn switch() {
    SCHED.switch();
}

static SCHED: Scheduler = Scheduler {
    inner: InitOnce::new()
};

pub unsafe fn enter() -> ! {
    SCHED.init();
    for i in 0..10 {
        SCHED.enqueue(SCHED.new_kernel(f0, i));
    }
    SCHED.enter();
}
