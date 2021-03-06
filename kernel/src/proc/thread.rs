//! Facilities for controlling threads - smallest units of
//! execution in the operating system
use crate::arch::aarch64::exception::ExceptionFrame;
use crate::proc::{
    wait::{Wait, WaitStatus},
    Process, ProcessRef, SCHED, THREADS,
};
use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use alloc::rc::Rc;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};
use libsys::{
    error::Errno,
    proc::{ExitCode, Pid, Tid},
    signal::Signal,
};

pub use crate::arch::platform::context::{self, Context};

/// Convenience wrapper for [Thread] references
pub type ThreadRef = Rc<Thread>;

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

struct ThreadInner {
    id: Tid,
    state: State,
    owner: Option<Pid>,
    pending_wait: Option<&'static Wait>,
    wait_status: WaitStatus,
    signal_entry: usize,
    signal_stack: usize,
}

/// Thread control data
pub struct Thread {
    inner: IrqSafeSpinLock<ThreadInner>,
    exit_wait: Wait,
    exit_status: InitOnce<ExitCode>,
    pub(super) ctx: UnsafeCell<Context>,
    signal_ctx: UnsafeCell<Context>,
    signal_pending: AtomicU32,
}

impl Thread {
    /// Returns currently active thread [Rc]-reference
    #[inline]
    pub fn current() -> ThreadRef {
        SCHED.current_thread()
    }

    /// Returns `true` if the thread is currently executing a signal handler context
    pub fn is_handling_signal(&self) -> bool {
        self.signal_pending.load(Ordering::Acquire) != 0
    }

    /// Returns a reference to thread `tid`, if it exists
    #[inline]
    pub fn get(tid: Tid) -> Option<ThreadRef> {
        THREADS.lock().get(&tid).cloned()
    }

    /// Returns the owner process
    #[inline]
    pub fn owner(&self) -> Option<ProcessRef> {
        self.inner.lock().owner.and_then(Process::get)
    }

    /// Returns [Pid] of the owner process
    pub fn owner_id(&self) -> Option<Pid> {
        self.inner.lock().owner
    }

    /// Creates a new kernel process
    pub fn new_kernel(
        owner: Option<Pid>,
        entry: extern "C" fn(usize) -> !,
        arg: usize,
    ) -> Result<ThreadRef, Errno> {
        let id = new_tid();

        let res = Rc::new(Self {
            ctx: UnsafeCell::new(Context::kernel(entry as usize, arg)),
            signal_ctx: UnsafeCell::new(Context::empty()),
            signal_pending: AtomicU32::new(0),
            exit_wait: Wait::new("thread_exit"),
            exit_status: InitOnce::new(),
            inner: IrqSafeSpinLock::new(ThreadInner {
                signal_entry: 0,
                signal_stack: 0,
                id,
                owner,
                pending_wait: None,
                wait_status: WaitStatus::Done,
                state: State::Ready,
            }),
        });
        debugln!("New kernel thread: {:?}", id);
        assert!(THREADS.lock().insert(id, res.clone()).is_none());
        Ok(res)
    }

    /// Creates a new userspace process
    pub fn new_user(
        owner: Pid,
        entry: usize,
        stack: usize,
        arg: usize,
        ttbr0: usize,
    ) -> Result<ThreadRef, Errno> {
        let id = new_tid();

        let res = Rc::new(Self {
            ctx: UnsafeCell::new(Context::user(entry, arg, ttbr0, stack)),
            signal_ctx: UnsafeCell::new(Context::empty()),
            signal_pending: AtomicU32::new(0),
            exit_wait: Wait::new("thread_exit"),
            exit_status: InitOnce::new(),
            inner: IrqSafeSpinLock::new(ThreadInner {
                signal_entry: 0,
                signal_stack: 0,
                id,
                owner: Some(owner),
                pending_wait: None,
                wait_status: WaitStatus::Done,
                state: State::Ready,
            }),
        });
        debugln!("New userspace thread: {:?}", id);
        assert!(THREADS.lock().insert(id, res.clone()).is_none());
        Ok(res)
    }

    /// Creates a fork thread cloning `frame` context
    pub fn fork(
        owner: Option<Pid>,
        frame: &ExceptionFrame,
        ttbr0: usize,
    ) -> Result<ThreadRef, Errno> {
        let id = new_tid();

        let res = Rc::new(Self {
            ctx: UnsafeCell::new(Context::fork(frame, ttbr0)),
            signal_ctx: UnsafeCell::new(Context::empty()),
            signal_pending: AtomicU32::new(0),
            exit_wait: Wait::new("thread_exit"),
            exit_status: InitOnce::new(),
            inner: IrqSafeSpinLock::new(ThreadInner {
                signal_entry: 0,
                signal_stack: 0,
                id,
                owner,
                pending_wait: None,
                wait_status: WaitStatus::Done,
                state: State::Ready,
            }),
        });
        debugln!("Forked new user thread: {:?}", id);
        assert!(THREADS.lock().insert(id, res.clone()).is_none());
        Ok(res)
    }

    /// Returns the thread ID
    #[inline]
    pub fn id(&self) -> Tid {
        self.inner.lock().id
    }

    /// Schedules an initial thread for execution
    ///
    /// # Safety
    ///
    /// Unsafe: only allowed to be called once, repeated calls
    ///         will generate undefined behavior
    pub unsafe fn enter(thread: ThreadRef) -> ! {
        // FIXME use some global lock to guarantee atomicity of thread entry?
        thread.inner.lock().state = State::Running;
        thread.current_context().enter()
    }

    /// Schedules a next thread for execution
    ///
    /// # Safety
    ///
    /// Unsafe:
    ///
    /// * Does not ensure src and dst threads are not the same thread
    /// * Does not ensure src is actually current context
    pub unsafe fn switch(src: ThreadRef, dst: ThreadRef, discard: bool) {
        {
            let mut src_lock = src.inner.lock();
            let mut dst_lock = dst.inner.lock();

            if !discard {
                assert_eq!(src_lock.state, State::Running);
                src_lock.state = State::Ready;
            }
            // assert!(dst_lock.state == State::Ready || dst_lock.state == State::Waiting);
            dst_lock.state = State::Running;
        }

        let src_ctx = src.current_context();
        let dst_ctx = dst.current_context();

        (&mut *src_ctx).switch(&mut *dst_ctx);
    }

    #[allow(clippy::mut_from_ref)]
    fn current_context(&self) -> &mut Context {
        if self.signal_pending.load(Ordering::Acquire) != 0 {
            unsafe { &mut *self.signal_ctx.get() }
        } else {
            unsafe { &mut *self.ctx.get() }
        }
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
    pub fn setup_wait(&self, wait: *const Wait) {
        #![allow(clippy::not_unsafe_ptr_arg_deref)]
        let mut lock = self.inner.lock();
        // FIXME this is not cool
        lock.pending_wait = Some(unsafe { &*wait });
        lock.wait_status = WaitStatus::Pending;
    }

    /// Suspends current thread until thread `tid` terminates
    pub fn waittid(tid: Tid) -> Result<(), Errno> {
        loop {
            let thread = THREADS
                .lock()
                .get(&tid)
                .cloned()
                .ok_or(Errno::DoesNotExist)?;

            if thread.state() == State::Finished {
                // TODO remove thread from its parent?
                return Ok(());
            }

            thread.exit_wait.wait(None)?;
        }
    }

    /// Updates pending wait status
    pub fn set_wait_status(&self, status: WaitStatus) {
        let mut lock = self.inner.lock();
        lock.wait_status = status;
    }

    /// Resets wait channel back to initial state
    pub fn reset_wait(&self) {
        let mut lock = self.inner.lock();
        lock.pending_wait = None;
        lock.wait_status = WaitStatus::Done;
    }

    /// Returns status of the thread's pending wait
    pub fn wait_status(&self) -> WaitStatus {
        self.inner.lock().wait_status
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

    /// Returns the thread state
    #[inline]
    pub fn state(&self) -> State {
        self.inner.lock().state
    }

    /// Sets the thread's owner process ID
    pub fn set_owner(&self, pid: Pid) {
        self.inner.lock().owner = Some(pid);
    }

    /// Sets up values needed for signal entry
    pub fn set_signal_entry(&self, entry: usize, stack: usize) {
        let mut lock = self.inner.lock();
        lock.signal_entry = entry;
        lock.signal_stack = stack;
    }

    /// Sets up a context for signal handler
    pub fn setup_signal(self: ThreadRef, signal: Signal, ttbr0: usize) {
        if self
            .signal_pending
            .compare_exchange_weak(0, signal as u32, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            panic!("Already handling a signal (maybe handle this case)");
        }

        let lock = self.inner.lock();
        if lock.signal_entry == 0 || lock.signal_stack == 0 {
            drop(lock);
            Process::exit_thread(self, ExitCode::from(-1));
            return;
        }

        let signal_ctx = unsafe { &mut *self.signal_ctx.get() };

        debugln!(
            "Signal entry: tid={:?}, pc={:#x}, sp={:#x}, ttbr0={:#x}",
            lock.id,
            lock.signal_entry,
            lock.signal_stack,
            ttbr0
        );

        unsafe {
            signal_ctx.setup_signal_entry(
                lock.signal_entry,
                signal as usize,
                ttbr0,
                lock.signal_stack,
            );
        }
    }

    /// Switches process main thread to a signal handler
    pub fn enter_signal(self: ThreadRef, signal: Signal, ttbr0: usize) {
        let src_ctx = self.ctx.get();
        let signal_ctx = unsafe { &mut *self.signal_ctx.get() };

        assert_eq!(self.state(), State::Running);
        self.setup_signal(signal, ttbr0);

        unsafe {
            (&mut *src_ctx).switch(signal_ctx);
        }
    }

    /// Interrupts pending wait (from signal routines)
    pub fn interrupt_wait(&self, enqueue: bool) {
        let mut lock = self.inner.lock();
        let tid = lock.id;
        let wait = lock.pending_wait.take();
        drop(lock);
        if let Some(wait) = wait {
            wait.abort(tid, enqueue);
        }
    }

    /// Cleans up any resources of the thread and aborts
    /// pending wait, if any
    pub fn terminate(&self, status: ExitCode) {
        let mut lock = self.inner.lock();
        lock.state = State::Finished;
        let tid = lock.id;
        let wait = lock.pending_wait.take();
        drop(lock);
        if let Some(wait) = wait {
            wait.abort(tid, false);
        }
        self.exit_status.init(status);
        self.exit_wait.wakeup_all();
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        debugln!("Dropping process {:?}", self.id());
    }
}

/// Allocates a new thread ID
pub fn new_tid() -> Tid {
    static LAST: AtomicU32 = AtomicU32::new(0);
    let id = LAST.fetch_add(1, Ordering::Relaxed);
    Tid::from(id)
}
