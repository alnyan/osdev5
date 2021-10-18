//! Process and thread manipulation facilities

use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::{MapAttributes, Space},
};
use crate::sync::IrqSafeNullLock;
use crate::util::InitOnce;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::rc::Rc;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};

pub use crate::arch::platform::context::{self, Context};

/// Wrapper type for a process struct reference
pub type ProcessRef = Rc<UnsafeCell<Process>>;

/// Structure describing an operating system process
#[allow(dead_code)]
pub struct Process {
    ctx: Context,
    space: &'static mut Space,
    id: u32,
}

struct SchedulerInner {
    // TODO the process list itself is not a scheduler-related thing so maybe
    //      move it outside?
    processes: BTreeMap<u32, ProcessRef>,
    queue: VecDeque<u32>,
    idle: u32,
    current: Option<u32>,
}

/// Process scheduler state and queues
pub struct Scheduler {
    inner: InitOnce<IrqSafeNullLock<SchedulerInner>>,
}

impl SchedulerInner {
    fn new_kernel(&mut self, entry: usize, arg: usize) -> u32 {
        static LAST_PID: AtomicU32 = AtomicU32::new(0);
        const USTACK_PAGE_COUNT: usize = 8;
        const USTACK_VIRT_TOP: usize = 0x100000000;
        const USTACK_VIRT_BASE: usize = USTACK_VIRT_TOP - USTACK_PAGE_COUNT * mem::PAGE_SIZE;

        let id = LAST_PID.fetch_add(1, Ordering::Relaxed);
        if id == 256 {
            panic!("Ran out of ASIDs (TODO FIXME)");
        }
        let space = Space::alloc_empty().unwrap();

        for i in 0..USTACK_PAGE_COUNT {
            let page = phys::alloc_page(PageUsage::Kernel).unwrap();
            space
                .map(
                    USTACK_VIRT_BASE + i * mem::PAGE_SIZE,
                    page,
                    MapAttributes::SH_OUTER
                        | MapAttributes::NOT_GLOBAL
                        | MapAttributes::UXN
                        | MapAttributes::PXN,
                )
                .unwrap();
        }

        let proc = Process {
            ctx: Context::kernel(
                entry,
                arg,
                ((space as *mut _ as usize) - mem::KERNEL_OFFSET) | ((id as usize) << 48),
                USTACK_VIRT_TOP,
            ),
            space,
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

        this.idle = this.new_kernel(idle_fn as usize, 0);

        this
    }
}

impl Scheduler {
    /// Constructs a new kernel-space process with `entry` and `arg`.
    /// Returns resulting process ID
    // TODO see the first TODO here
    pub fn new_kernel(&self, entry: usize, arg: usize) -> u32 {
        self.inner.get().lock().new_kernel(entry, arg)
    }

    /// Initializes inner data structure:
    ///
    /// * idle thread
    /// * process list/queue data structs
    pub fn init(&self) {
        self.inner.init(IrqSafeNullLock::new(SchedulerInner::new()));
    }

    /// Schedules a thread for execution
    pub fn enqueue(&self, pid: u32) {
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
                inner.idle
            } else {
                inner.queue.pop_front().unwrap()
            };

            inner.current = Some(id);
            inner.processes.get(&id).unwrap().clone()
        };

        (*thread.get()).ctx.enter();
    }

    /// Switches to the next task scheduled for execution. If there're
    /// none present in the queue, switches to the idle task.
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
                inner.processes.get(&next).unwrap().clone(),
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

#[inline(never)]
extern "C" fn f1(u: usize) {
    let mut x = u;
    while x != 0 {
        cortex_a::asm::nop();
        x -= 1;
    }
}

#[inline(never)]
extern "C" fn f0(a: usize) -> ! {
    loop {
        unsafe {
            asm!("svc #0", in("x0") a, in("x1") &a);
        }
        f1(10000000);
    }
}

/// Performs a task switch.
///
/// See [Scheduler::switch]
pub fn switch() {
    SCHED.switch();
}

static SCHED: Scheduler = Scheduler {
    inner: InitOnce::new(),
};

/// Sets up initial process and enters it.
///
/// See [Scheduler::enter]
///
/// # Safety
///
/// Unsafe: May only be called once.
pub unsafe fn enter() -> ! {
    SCHED.init();
    for i in 0..4 {
        SCHED.enqueue(SCHED.new_kernel(f0 as usize, i));
    }
    SCHED.enter();
}
