//!
use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::{MapAttributes, Space},
};
use alloc::boxed::Box;
use alloc::rc::Rc;
use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicU32, Ordering};
use error::Errno;
use crate::proc::{PROCESSES, SCHED};

pub use crate::arch::platform::context::{self, Context};

/// Wrapper type for a process struct reference
pub type ProcessRef = Rc<UnsafeCell<Process>>;

/// Wrapper type for process ID
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq)]
#[repr(transparent)]
pub struct Pid(u32);

/// List of possible process states
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
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
    ///
    pub state: State,
    id: Pid,
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

impl Process {
    ///
    pub unsafe fn enter(&mut self) -> ! {
        self.ctx.enter()
    }

    ///
    pub unsafe fn switch_to(&mut self, dst: *mut Process) {
        self.ctx.switch(&mut (*dst).ctx);
    }

    ///
    pub const fn id(&self) -> Pid {
        self.id
    }

    ///
    pub fn new_kernel(entry: extern "C" fn (usize) -> !, arg: usize) -> Result<ProcessRef, Errno> {
        let id = Pid::new_kernel();
        let res = Rc::new(UnsafeCell::new(Self {
            ctx: Context::kernel(entry as usize, arg),
            id,
            space: None,
            state: State::Ready
        }));
        assert!(PROCESSES.lock().insert(id, res.clone()).is_none());
        Ok(res)
    }

    /// Terminates a process.
    ///
    /// # Safety
    ///
    /// Unsafe: only allowed to be called on "self" process at this moment.
    pub unsafe fn exit(&mut self) -> ! {
        infoln!("Process {} is exiting", self.id);
        self.state = State::Finished;
        SCHED.switch(true);
        panic!("This code should never run");
    }

    /// Loads a new program into process address space
    pub fn execve<F: FnOnce(&mut Space) -> Result<usize, Errno>>(
        &mut self,
        loader: F,
        arg: usize,
    ) -> Result<(), Errno> {
        todo!()
        // unsafe {
        //     // Run with interrupts disabled
        //     asm!("msr daifset, #2");
        // }

        // let id = if self.id.is_kernel() {
        //     let r = Pid::new_user();
        //     debugln!(
        //         "Process downgrades from kernel to user: {} -> {}",
        //         self.id,
        //         r
        //     );
        //     r
        // } else {
        //     self.id
        // };

        // let ustack_pages = 4;
        // let new_space = Space::alloc_empty()?;
        // let new_space_phys = (new_space as *mut _ as usize) - mem::KERNEL_OFFSET;

        // let ustack_virt_bottom = SchedulerInner::USTACK_VIRT_TOP - ustack_pages * mem::PAGE_SIZE;
        // for i in 0..ustack_pages {
        //     let page = phys::alloc_page(PageUsage::UserPrivate).unwrap();
        //     let flags = MapAttributes::SH_OUTER
        //         | MapAttributes::NOT_GLOBAL
        //         | MapAttributes::UXN
        //         | MapAttributes::PXN
        //         | MapAttributes::AP_BOTH_READWRITE;
        //     new_space
        //         .map(ustack_virt_bottom + i * mem::PAGE_SIZE, page, flags)
        //         .unwrap();
        // }

        // let entry = loader(new_space)?;

        // debugln!("Will now enter at {:#x}", entry);
        // self.ctx = Context::user(
        //     entry,
        //     arg,
        //     new_space_phys | ((id.asid() as usize) << 48),
        //     SchedulerInner::USTACK_VIRT_TOP,
        // );
        // self.space = Some(new_space);
        // // TODO drop old address space

        // unsafe {
        //     self.ctx.enter();
        // }
    }
}
