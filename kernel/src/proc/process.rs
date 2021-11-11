//! Process data and control
use crate::arch::aarch64::exception::ExceptionFrame;
use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::{MapAttributes, Space},
};
use crate::proc::{wait::Wait, ProcessIo, PROCESSES, SCHED};
use crate::sync::IrqSafeSpinLock;
use alloc::rc::Rc;
use core::cell::UnsafeCell;
use core::fmt;
use core::sync::atomic::{AtomicU32, Ordering};
use libsys::error::Errno;

pub use crate::arch::platform::context::{self, Context};

/// Wrapper type for a process struct reference
pub type ProcessRef = Rc<Process>;

/// Wrapper type for process exit code
#[derive(Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct ExitCode(i32);

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

struct ProcessInner {
    space: Option<&'static mut Space>,
    state: State,
    id: Pid,
    wait_flag: bool,
    exit: Option<ExitCode>,
}

/// Structure describing an operating system process
#[allow(dead_code)]
pub struct Process {
    ctx: UnsafeCell<Context>,
    inner: IrqSafeSpinLock<ProcessInner>,
    exit_wait: Wait,
    /// Process I/O context
    pub io: IrqSafeSpinLock<ProcessIo>,
}

impl From<i32> for ExitCode {
    fn from(f: i32) -> Self {
        Self(f)
    }
}

impl From<()> for ExitCode {
    fn from(_: ()) -> Self {
        Self(0)
    }
}

impl From<ExitCode> for i32 {
    fn from(f: ExitCode) -> Self {
        f.0
    }
}

impl Pid {
    /// Kernel idle process always has PID of zero
    pub const IDLE: Self = Self(Self::KERNEL_BIT);

    const KERNEL_BIT: u32 = 1 << 31;

    /// Constructs an instance of user-space PID
    pub const fn user(id: u32) -> Self {
        assert!(id < 256, "PID is too high");
        Self(id)
    }

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

    /// Returns bit value of this pid
    pub const fn value(self) -> u32 {
        self.0
    }

    pub const unsafe fn from_raw(num: u32) -> Self {
        Self(num)
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
    const USTACK_VIRT_TOP: usize = 0x100000000;
    const USTACK_PAGES: usize = 4;

    /// Returns currently executing process
    pub fn current() -> ProcessRef {
        SCHED.current_process()
    }

    pub fn get(pid: Pid) -> Option<ProcessRef> {
        PROCESSES.lock().get(&pid).cloned()
    }

    /// Schedules an initial thread for execution
    ///
    /// # Safety
    ///
    /// Unsafe: only allowed to be called once, repeated calls
    ///         will generate undefined behavior
    pub unsafe fn enter(proc: ProcessRef) -> ! {
        // FIXME use some global lock to guarantee atomicity of thread entry?
        proc.inner.lock().state = State::Running;
        let ctx = proc.ctx.get();

        (&mut *ctx).enter()
    }

    #[inline]
    pub fn manipulate_space<F: FnOnce(&mut Space) -> Result<(), Errno>>(
        &self,
        f: F,
    ) -> Result<(), Errno> {
        f(self.inner.lock().space.as_mut().unwrap())
    }

    /// Schedules a next thread for execution
    ///
    /// # Safety
    ///
    /// Unsafe:
    ///
    /// * Does not ensure src and dst threads are not the same thread
    /// * Does not ensure src is actually current context
    pub unsafe fn switch(src: ProcessRef, dst: ProcessRef, discard: bool) {
        {
            let mut src_lock = src.inner.lock();
            let mut dst_lock = dst.inner.lock();

            if !discard {
                assert_eq!(src_lock.state, State::Running);
                src_lock.state = State::Ready;
            }
            assert!(dst_lock.state == State::Ready || dst_lock.state == State::Waiting);
            dst_lock.state = State::Running;
        }

        let src_ctx = src.ctx.get();
        let dst_ctx = dst.ctx.get();

        (&mut *src_ctx).switch(&mut *dst_ctx);
    }

    /// Suspends current process with a "waiting" status
    pub fn enter_wait(&self) {
        let drop = {
            let mut lock = self.inner.lock();
            let drop = lock.state == State::Running;
            lock.state = State::Waiting;
            SCHED.dequeue(lock.id);
            drop
        };
        if drop {
            SCHED.switch(true);
        }
    }

    /// Changes process wait condition status
    pub fn set_wait_flag(&self, v: bool) {
        self.inner.lock().wait_flag = v;
    }

    /// Returns `true` if process wait condition has not been reached
    pub fn wait_flag(&self) -> bool {
        self.inner.lock().wait_flag
    }

    /// Returns the process ID
    pub fn id(&self) -> Pid {
        self.inner.lock().id
    }

    /// Creates a new kernel process
    pub fn new_kernel(entry: extern "C" fn(usize) -> !, arg: usize) -> Result<ProcessRef, Errno> {
        let id = Pid::new_kernel();
        let res = Rc::new(Self {
            ctx: UnsafeCell::new(Context::kernel(entry as usize, arg)),
            io: IrqSafeSpinLock::new(ProcessIo::new()),
            exit_wait: Wait::new(),
            inner: IrqSafeSpinLock::new(ProcessInner {
                id,
                exit: None,
                space: None,
                wait_flag: false,
                state: State::Ready,
            }),
        });
        debugln!("New kernel process: {}", id);
        assert!(PROCESSES.lock().insert(id, res.clone()).is_none());
        Ok(res)
    }

    /// Creates a "fork" of the process, cloning its address space and
    /// resources
    pub fn fork(&self, frame: &mut ExceptionFrame) -> Result<Pid, Errno> {
        let src_io = self.io.lock();
        let mut src_inner = self.inner.lock();

        let dst_id = Pid::new_user();
        let dst_space = src_inner.space.as_mut().unwrap().fork()?;
        let dst_space_phys = (dst_space as *mut _ as usize) - mem::KERNEL_OFFSET;
        let dst_ttbr0 = dst_space_phys | ((dst_id.asid() as usize) << 48);

        let dst = Rc::new(Self {
            ctx: UnsafeCell::new(Context::fork(frame, dst_ttbr0)),
            io: IrqSafeSpinLock::new(src_io.fork()?),
            exit_wait: Wait::new(),
            inner: IrqSafeSpinLock::new(ProcessInner {
                id: dst_id,
                exit: None,
                space: Some(dst_space),
                state: State::Ready,
                wait_flag: false,
            }),
        });
        debugln!("Process {} forked into {}", src_inner.id, dst_id);
        assert!(PROCESSES.lock().insert(dst_id, dst).is_none());
        SCHED.enqueue(dst_id);

        Ok(dst_id)
    }

    /// Terminates a process.
    pub fn exit<I: Into<ExitCode>>(&self, status: I) {
        let status = status.into();
        let drop = {
            let mut lock = self.inner.lock();
            let drop = lock.state == State::Running;
            infoln!("Process {} is exiting: {:?}", lock.id, status);
            assert!(lock.exit.is_none());
            lock.exit = Some(status);
            lock.state = State::Finished;

            if let Some(space) = lock.space.take() {
                unsafe {
                    Space::release(space);
                    asm!("tlbi aside1, {}", in(reg) ((lock.id.asid() as usize) << 48));
                }
            }

            self.io.lock().handle_exit();

            SCHED.dequeue(lock.id);
            drop
        };
        self.exit_wait.wakeup_all();
        if drop {
            SCHED.switch(true);
            panic!("This code should never run");
        }
    }

    fn collect(&self) -> Option<ExitCode> {
        let lock = self.inner.lock();
        if lock.state == State::Finished {
            lock.exit
        } else {
            None
        }
    }

    /// Waits for a process to finish and reaps it
    pub fn waitpid(pid: Pid) -> Result<ExitCode, Errno> {
        loop {
            let proc = PROCESSES
                .lock()
                .get(&pid)
                .cloned()
                .ok_or(Errno::DoesNotExist)?;

            if let Some(r) = proc.collect() {
                // TODO drop the process struct itself
                PROCESSES.lock().remove(&proc.id());
                debugln!("pid {} has {} refs", proc.id(), Rc::strong_count(&proc));
                return Ok(r);
            }

            proc.exit_wait.wait(None)?;
        }
    }

    /// Loads a new program into current process address space
    pub fn execve<F: FnOnce(&mut Space) -> Result<usize, Errno>>(
        loader: F,
        arg: usize,
    ) -> Result<(), Errno> {
        unsafe {
            // Run with interrupts disabled
            asm!("msr daifset, #2");
        }

        let proc = SCHED.current_process();
        let mut lock = proc.inner.lock();
        if lock.id.is_kernel() {
            let mut proc_lock = PROCESSES.lock();
            let old_pid = lock.id;
            assert!(
                proc_lock.remove(&old_pid).is_some(),
                "Failed to downgrade kernel process (remove kernel pid)"
            );
            lock.id = Pid::new_user();
            debugln!(
                "Process downgrades from kernel to user: {} -> {}",
                old_pid,
                lock.id
            );
            assert!(proc_lock.insert(lock.id, proc.clone()).is_none());
            unsafe {
                SCHED.hack_current_pid(lock.id);
            }
        } else {
            // Invalidate user ASID
            let input = (lock.id.asid() as usize) << 48;
            unsafe {
                asm!("tlbi aside1, {}", in(reg) input);
            }
        }

        proc.io.lock().handle_cloexec();

        let new_space = Space::alloc_empty()?;
        let new_space_phys = (new_space as *mut _ as usize) - mem::KERNEL_OFFSET;

        let ustack_virt_bottom = Self::USTACK_VIRT_TOP - Self::USTACK_PAGES * mem::PAGE_SIZE;
        for i in 0..Self::USTACK_PAGES {
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
        // TODO drop old address space
        lock.space = Some(new_space);

        unsafe {
            // TODO drop old context
            let ctx = proc.ctx.get();

            ctx.write(Context::user(
                entry,
                arg,
                new_space_phys | ((lock.id.asid() as usize) << 48),
                Self::USTACK_VIRT_TOP,
            ));

            assert_eq!(lock.state, State::Running);

            drop(lock);

            (*ctx).enter();
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        debugln!("Dropping process {}", self.id());
    }
}
