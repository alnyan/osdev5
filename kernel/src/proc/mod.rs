//! Process and thread manipulation facilities

use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::{MapAttributes, Space},
};
use crate::sync::IrqSafeNullLock;
use crate::util::InitOnce;
use alloc::boxed::Box;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::rc::Rc;
use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicU32, Ordering};
use error::Errno;

pub use crate::arch::platform::context::{self, Context};

pub mod elf;

/// Wrapper type for a process struct reference
pub type ProcessRef = Rc<UnsafeCell<Process>>;

/// Wrapper type for process ID
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Pid(u32);

/// List of possible process states
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessState {
    /// Process is ready to be executed and/or is scheduled for it
    Ready,
    /// Process is currently running or is in system call/interrupt handler
    Running,
    /// Process has finished execution and is waiting to be reaped
    Finished,
    /// Process is waiting for some external event
    Waiting,
}

/// Structure describing an operating system process
#[allow(dead_code)]
pub struct Process {
    ctx: Context,
    // TODO move to Option<Box<>>ed user data struct
    space: Option<&'static mut Space>,
    state: ProcessState,
    id: Pid,
}

struct SchedulerInner {
    // TODO the process list itself is not a scheduler-related thing so maybe
    //      move it outside?
    processes: BTreeMap<Pid, ProcessRef>,
    queue: VecDeque<Pid>,
    idle: Option<Pid>,
    current: Option<Pid>,
}

/// Process scheduler state and queues
pub struct Scheduler {
    inner: InitOnce<IrqSafeNullLock<SchedulerInner>>,
}

impl Pid {
    /// Kernel idle process always has PID of zero
    pub const IDLE: Self = Self(0 | Self::KERNEL_BIT);

    const KERNEL_BIT: u32 = 1 << 31;

    /// Allocates a new kernel-space PID
    pub fn new_kernel() -> Self {
        static LAST: AtomicU32 = AtomicU32::new(0);
        let id = LAST.fetch_add(1, Ordering::Relaxed);
        assert!(id & Self::KERNEL_BIT == 0, "Out of kernel PIDs");
        Self(id | Self::KERNEL_BIT)
    }

    /// Allocates a new user-space PID.
    ///
    /// First user PID is #1.
    pub fn new_user() -> Self {
        static LAST: AtomicU32 = AtomicU32::new(1);
        let id = LAST.fetch_add(1, Ordering::Relaxed);
        assert!(id < 256, "Out of user PIDs");
        Self(id)
    }

    /// Returns `true` if this PID belongs to a kernel process
    pub fn is_kernel(self) -> bool {
        self.0 & Self::KERNEL_BIT != 0
    }

    /// Returns address space ID of a user-space process.
    ///
    /// Panics if called on kernel process PID.
    pub fn asid(self) -> u8 {
        assert!(!self.is_kernel());
        self.0 as u8
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Pid(#{}{})",
            if self.is_kernel() { "K" } else { "U" },
            self.0 & !Self::KERNEL_BIT
        )
    }
}

impl SchedulerInner {
    const USTACK_VIRT_TOP: usize = 0x100000000;

    fn new_kernel(&mut self, entry: extern "C" fn(usize) -> !, arg: usize) -> Pid {
        let id = Pid::new_kernel();

        let proc = Process {
            ctx: Context::kernel(entry as usize, arg),
            space: None,
            state: ProcessState::Ready,
            id,
        };
        debugln!("Created kernel process: {}", id);

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
            idle: None,
            current: None,
        };

        this.idle = Some(this.new_kernel(idle_fn, 0));

        this
    }
}

impl Scheduler {
    /// Constructs a new kernel-space process with `entry` and `arg`.
    /// Returns resulting process ID
    // TODO see the first TODO here
    pub fn new_kernel(&self, entry: extern "C" fn(usize) -> !, arg: usize) -> Pid {
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
            let proc = inner.processes.get(&id).unwrap().clone();
            (*proc.get()).state = ProcessState::Running;
            proc
        };

        (*thread.get()).ctx.enter();
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
            let from = inner.processes.get(&current).unwrap().clone();
            let to = inner.processes.get(&next).unwrap().clone();

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
                (*from.get()).ctx.switch(&mut (*to.get()).ctx);
            }
        }
    }

    /// Returns a Rc-reference to currently running process
    pub fn current_process(&self) -> ProcessRef {
        let inner = self.inner.get().lock();
        let current = inner.current.unwrap();
        inner.processes.get(&current).unwrap().clone()
    }
}

impl Process {
    /// Returns current process Rc-reference.
    ///
    /// See [Scheduler::current_process].
    #[inline]
    pub fn this() -> ProcessRef {
        SCHED.current_process()
    }

    /// Terminates a process.
    ///
    /// # Safety
    ///
    /// Unsafe: only allowed to be called on "self" process at this moment.
    pub unsafe fn exit(&mut self) -> ! {
        self.state = ProcessState::Finished;
        SCHED.switch(true);
        panic!("This code should never run");
    }

    /// Loads a new program into process address space
    pub fn execve<F: FnOnce(&mut Space) -> Result<usize, Errno>>(
        &mut self,
        loader: F,
        arg: usize,
    ) -> Result<(), Errno> {
        unsafe {
            // Run with interrupts disabled
            asm!("msr daifset, #2");
        }

        let id = if self.id.is_kernel() {
            let r = Pid::new_user();
            debugln!(
                "Process downgrades from kernel to user: {} -> {}",
                self.id,
                r
            );
            r
        } else {
            self.id
        };

        let ustack_pages = 4;
        let new_space = Space::alloc_empty()?;
        let new_space_phys = (new_space as *mut _ as usize) - mem::KERNEL_OFFSET;

        let ustack_virt_bottom = SchedulerInner::USTACK_VIRT_TOP - ustack_pages * mem::PAGE_SIZE;
        for i in 0..ustack_pages {
            let page = phys::alloc_page(PageUsage::UserPrivate).unwrap();
            let flags = MapAttributes::SH_OUTER
                | MapAttributes::NOT_GLOBAL
                | MapAttributes::UXN
                | MapAttributes::PXN
                | MapAttributes::AP_BOTH_READWRITE;
            new_space
                .map(ustack_virt_bottom + i * mem::PAGE_SIZE, page, flags)
                .unwrap();
        }

        let entry = loader(new_space)?;

        debugln!("Will now enter at {:#x}", entry);
        self.ctx = Context::user(
            entry,
            arg,
            new_space_phys | ((id.asid() as usize) << 48),
            SchedulerInner::USTACK_VIRT_TOP,
        );
        self.space = Some(new_space);
        // TODO drop old address space

        unsafe {
            self.ctx.enter();
        }
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
            unsafe { (*Process::this().get()).exit() }
        }

        $crate::proc::SCHED.enqueue($crate::proc::SCHED.new_kernel(__inner_func, $src_arg));
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
            let proc = unsafe { &mut *SCHED.current_process().get() };
            proc.execve(|space| elf::load_elf(space, start as *const u8), 0)
                .unwrap();
            panic!("This code should not run");
        }, initrd as usize);
    }
    SCHED.enter();
}
