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
use core::sync::atomic::{AtomicU32, Ordering};
use libsys::{error::Errno, signal::Signal, proc::{ExitCode, Pid}};

pub use crate::arch::platform::context::{self, Context};

/// Wrapper type for a process struct reference
pub type ProcessRef = Rc<Process>;

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
    signal_entry: usize,
    signal_stack: usize,
}

/// Structure describing an operating system process
#[allow(dead_code)]
pub struct Process {
    ctx: UnsafeCell<Context>,
    signal_ctx: UnsafeCell<Context>,
    inner: IrqSafeSpinLock<ProcessInner>,
    exit_wait: Wait,
    signal_state: AtomicU32,
    signal_pending: AtomicU32,
    /// Process I/O context
    pub io: IrqSafeSpinLock<ProcessIo>,
}

impl Process {
    const USTACK_VIRT_TOP: usize = 0x100000000;
    const USTACK_PAGES: usize = 4;

    /// Returns currently executing process
    pub fn current() -> ProcessRef {
        SCHED.current_process()
    }

    /// Returns process (if any) to which `pid` refers
    pub fn get(pid: Pid) -> Option<ProcessRef> {
        PROCESSES.lock().get(&pid).cloned()
    }

    /// Sets a pending signal for a process
    pub fn set_signal(&self, signal: Signal) {
        let lock = self.inner.lock();

        match lock.state {
            State::Running => {
                drop(lock);
                self.enter_signal(signal);
            }
            State::Waiting => {
                // TODO abort whatever the process is waiting for
                todo!()
            }
            State::Ready => {
                todo!()
            }
            State::Finished => {
                // TODO report error back
                todo!()
            }
        }
    }

    /// Switches current thread back from signal handler
    pub fn return_from_signal(&self) {
        if self.signal_pending.load(Ordering::Acquire) == 0 {
            panic!("TODO handle cases when returning from no signal");
        }
        self.signal_pending.store(0, Ordering::Release);

        let src_ctx = self.signal_ctx.get();
        let dst_ctx = self.ctx.get();

        assert_eq!(self.inner.lock().state, State::Running);

        unsafe {
            (&mut *src_ctx).switch(&mut *dst_ctx);
        }
    }

    /// Switches current thread to a signal handler
    pub fn enter_signal(&self, signal: Signal) {
        if self
            .signal_pending
            .compare_exchange_weak(0, signal as u32, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            panic!("Already handling a signal (maybe handle this case)");
        }

        let mut lock = self.inner.lock();
        let signal_ctx = unsafe { &mut *self.signal_ctx.get() };

        let dst_id = lock.id;
        let dst_space_phys = lock.space.as_mut().unwrap().address_phys();
        let dst_ttbr0 = dst_space_phys | ((dst_id.asid() as usize) << 48);

        debugln!(
            "Signal entry: pc={:#x}, sp={:#x}, ttbr0={:#x}",
            lock.signal_entry,
            lock.signal_stack,
            dst_ttbr0
        );
        assert_eq!(lock.state, State::Running);

        unsafe {
            signal_ctx.setup_signal_entry(
                lock.signal_entry,
                signal as usize,
                dst_ttbr0,
                lock.signal_stack,
            );
        }
        let src_ctx = self.ctx.get();
        drop(lock);

        unsafe {
            (&mut *src_ctx).switch(signal_ctx);
        }
    }

    /// Sets up values needed for signal entry
    pub fn setup_signal_context(&self, entry: usize, stack: usize) {
        let mut lock = self.inner.lock();
        lock.signal_entry = entry;
        lock.signal_stack = stack;
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
        proc.current_context().enter()
    }

    /// Executes a function allowing mutation of the process address space
    #[inline]
    pub fn manipulate_space<F: FnOnce(&mut Space) -> Result<(), Errno>>(
        &self,
        f: F,
    ) -> Result<(), Errno> {
        f(self.inner.lock().space.as_mut().unwrap())
    }

    #[allow(clippy::mut_from_ref)]
    fn current_context(&self) -> &mut Context {
        if self.signal_pending.load(Ordering::Acquire) != 0 {
            unsafe { &mut *self.signal_ctx.get() }
        } else {
            unsafe { &mut *self.ctx.get() }
        }
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

        let src_ctx = src.current_context();
        let dst_ctx = dst.current_context();

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
        let id = new_kernel_pid();
        let res = Rc::new(Self {
            ctx: UnsafeCell::new(Context::kernel(entry as usize, arg)),
            signal_ctx: UnsafeCell::new(Context::empty()),
            io: IrqSafeSpinLock::new(ProcessIo::new()),
            exit_wait: Wait::new(),
            signal_state: AtomicU32::new(0),
            signal_pending: AtomicU32::new(0),
            inner: IrqSafeSpinLock::new(ProcessInner {
                signal_entry: 0,
                signal_stack: 0,
                id,
                exit: None,
                space: None,
                wait_flag: false,
                state: State::Ready,
            }),
        });
        debugln!("New kernel process: {:?}", id);
        assert!(PROCESSES.lock().insert(id, res.clone()).is_none());
        Ok(res)
    }

    /// Creates a "fork" of the process, cloning its address space and
    /// resources
    pub fn fork(&self, frame: &mut ExceptionFrame) -> Result<Pid, Errno> {
        let src_io = self.io.lock();
        let mut src_inner = self.inner.lock();

        let dst_id = new_user_pid();
        let dst_space = src_inner.space.as_mut().unwrap().fork()?;
        let dst_space_phys = (dst_space as *mut _ as usize) - mem::KERNEL_OFFSET;
        let dst_ttbr0 = dst_space_phys | ((dst_id.asid() as usize) << 48);

        let dst = Rc::new(Self {
            ctx: UnsafeCell::new(Context::fork(frame, dst_ttbr0)),
            signal_ctx: UnsafeCell::new(Context::empty()),
            io: IrqSafeSpinLock::new(src_io.fork()?),
            exit_wait: Wait::new(),
            signal_state: AtomicU32::new(0),
            signal_pending: AtomicU32::new(0),
            inner: IrqSafeSpinLock::new(ProcessInner {
                signal_entry: 0,
                signal_stack: 0,
                id: dst_id,
                exit: None,
                space: Some(dst_space),
                state: State::Ready,
                wait_flag: false,
            }),
        });
        debugln!("Process {:?} forked into {:?}", src_inner.id, dst_id);
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
            infoln!("Process {:?} is exiting: {:?}", lock.id, status);
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
                debugln!("pid {:?} has {} refs", proc.id(), Rc::strong_count(&proc));
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
            lock.id = new_user_pid();
            debugln!(
                "Process downgrades from kernel to user: {:?} -> {:?}",
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
        debugln!("Dropping process {:?}", self.id());
    }
}

/// Allocates a new kernel-space PID
pub fn new_kernel_pid() -> Pid {
    static LAST: AtomicU32 = AtomicU32::new(0);
    let id = LAST.fetch_add(1, Ordering::Relaxed);
    Pid::kernel(id)
}

/// Allocates a new user-space PID.
///
/// First user PID is #1.
pub fn new_user_pid() -> Pid {
    static LAST: AtomicU32 = AtomicU32::new(1);
    let id = LAST.fetch_add(1, Ordering::Relaxed);
    assert!(id < 256, "Out of user PIDs");
    Pid::user(id)
}
