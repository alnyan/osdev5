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
use alloc::boxed::Box;
use error::Errno;

pub use crate::arch::platform::context::{self, Context};

pub mod elf;

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

static LAST_PID: AtomicU32 = AtomicU32::new(0);
impl SchedulerInner {
    const USTACK_VIRT_TOP: usize = 0x100000000;

    fn new_kernel<F: FnOnce(&mut Space) -> Result<usize, Errno>>(
        &mut self,
        loader: F,
        ustack_pages: usize,
        arg: usize,
    ) -> u32 {
        let id = LAST_PID.fetch_add(1, Ordering::Relaxed);
        if id == 256 {
            panic!("Ran out of ASIDs (TODO FIXME)");
        }
        let space = Space::alloc_empty().unwrap();

        let ustack_virt_bottom = Self::USTACK_VIRT_TOP - ustack_pages * mem::PAGE_SIZE;
        for i in 0..ustack_pages {
            let page = phys::alloc_page(PageUsage::Kernel).unwrap();
            space
                .map(
                    ustack_virt_bottom + i * mem::PAGE_SIZE,
                    page,
                    MapAttributes::SH_OUTER
                        | MapAttributes::NOT_GLOBAL
                        | MapAttributes::UXN
                        | MapAttributes::PXN,
                )
                .unwrap();
        }

        let entry = loader(space).unwrap();

        let proc = Process {
            ctx: Context::kernel(
                entry,
                arg,
                ((space as *mut _ as usize) - mem::KERNEL_OFFSET) | ((id as usize) << 48),
                if ustack_pages != 0 {
                    Self::USTACK_VIRT_TOP
                } else {
                    0
                },
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

    fn new_idle(&mut self) -> u32 {
        self.new_kernel(|_| Ok(idle_fn as usize), 0, 0)
    }

    fn new() -> Self {
        let mut this = Self {
            processes: BTreeMap::new(),
            queue: VecDeque::new(),
            idle: 0,
            current: None,
        };

        this.idle = this.new_idle();

        this
    }
}

impl Scheduler {
    /// Constructs a new kernel-space process with `entry` and `arg`.
    /// Returns resulting process ID
    // TODO see the first TODO here
    pub fn new_kernel<F: FnOnce(&mut Space) -> Result<usize, Errno>>(
        &self,
        loader: F,
        ustack_pages: usize,
        arg: usize,
    ) -> u32 {
        self.inner
            .get()
            .lock()
            .new_kernel(loader, ustack_pages, arg)
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

    ///
    pub fn current_process(&self) -> ProcessRef {
        let inner = self.inner.get().lock();
        let current = inner.current.unwrap();
        inner.processes.get(&current).unwrap().clone()
    }
}

impl Process {
    ///
    pub fn execve<F: FnOnce(&mut Space) -> Result<usize, Errno>>(
        &mut self,
        loader: F,
        arg: usize,
    ) -> Result<(), Errno> {
        unsafe {
            // Run with interrupts disabled
            asm!("msr daifset, #2");
        }

        let ustack_pages = 4;
        let new_space = Space::alloc_empty()?;
        let new_space_phys = ((new_space as *mut _ as usize) - mem::KERNEL_OFFSET); // | ((id as usize) << 48),

        let ustack_virt_bottom = SchedulerInner::USTACK_VIRT_TOP - ustack_pages * mem::PAGE_SIZE;
        for i in 0..ustack_pages {
            let page = phys::alloc_page(PageUsage::Kernel).unwrap();
            new_space
                .map(
                    ustack_virt_bottom + i * mem::PAGE_SIZE,
                    page,
                    MapAttributes::SH_OUTER
                        | MapAttributes::NOT_GLOBAL
                        | MapAttributes::UXN
                        | MapAttributes::PXN,
                )
                .unwrap();
        }

        let entry = loader(new_space)?;

        self.ctx = Context::kernel(
            entry,
            0,
            new_space_phys | ((self.id as usize) << 48),
            SchedulerInner::USTACK_VIRT_TOP,
        );
        self.space = new_space;

        unsafe {
            self.ctx.enter();
        }
        panic!("This should not run");
    }
}

extern "C" fn idle_fn(_a: usize) -> ! {
    loop {}
}

#[inline(never)]
extern "C" fn init_fn(initrd_ptr: usize) -> ! {
    debugln!("Running kernel init process");

    let (start, _end) = unsafe { *(initrd_ptr as *const (usize, usize)) };
    let proc = unsafe { &mut *SCHED.current_process().get() };
    proc.execve(|space| elf::load_elf(space, start as *const u8), 0).unwrap();
    loop {}
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
pub unsafe fn enter(initrd: Option<(usize, usize)>) -> ! {
    SCHED.init();
    if let Some((start, end)) = initrd {
        let initrd = Box::into_raw(Box::new((mem::virtualize(start), mem::virtualize(end))));
        SCHED.enqueue(SCHED.new_kernel(|_| Ok(init_fn as usize), 0, initrd as usize));
    }
    SCHED.enter();
}
