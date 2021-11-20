//! Process data and control
use crate::arch::aarch64::exception::ExceptionFrame;
use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::{MapAttributes, Space},
};
use crate::proc::{
    wait::Wait, Context, ProcessIo, Thread, ThreadRef, ThreadState, PROCESSES, SCHED, THREADS,
};
use crate::sync::{IrqSafeSpinLock, IrqSafeSpinLockGuard};
use alloc::{rc::Rc, vec::Vec};
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU32, Ordering};
use libsys::{
    error::Errno,
    proc::{ExitCode, Pid},
    signal::Signal,
};

/// Wrapper type for a process struct reference
pub type ProcessRef = Rc<Process>;

/// List of possible process states
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ProcessState {
    /// Process is alive
    Active,
    /// Process has finished execution and is waiting to be reaped
    Finished,
}

struct ProcessInner {
    space: Option<&'static mut Space>,
    state: ProcessState,
    id: Pid,
    exit: Option<ExitCode>,
    threads: Vec<u32>,
}

/// Structure describing an operating system process
#[allow(dead_code)]
pub struct Process {
    inner: IrqSafeSpinLock<ProcessInner>,
    exit_wait: Wait,
    signal_state: AtomicU32,
    /// Process I/O context
    pub io: IrqSafeSpinLock<ProcessIo>,
}

impl Process {
    const USTACK_VIRT_TOP: usize = 0x100000000;
    const USTACK_PAGES: usize = 4;

    #[inline]
    pub fn id(&self) -> Pid {
        self.inner.lock().id
    }

    #[inline]
    pub fn current() -> ProcessRef {
        Thread::current().owner().unwrap()
    }

    #[inline]
    pub fn manipulate_space<F>(&self, f: F) -> Result<(), Errno>
    where
        F: FnOnce(&mut Space) -> Result<(), Errno>,
    {
        f(self.inner.lock().space.as_mut().unwrap())
    }

    pub fn new_kernel(entry: extern "C" fn(usize) -> !, arg: usize) -> Result<ProcessRef, Errno> {
        let id = new_kernel_pid();
        let thread = Thread::new_kernel(Some(id), entry, arg)?;
        let mut inner = ProcessInner {
            threads: Vec::new(),
            id,
            exit: None,
            space: None,
            state: ProcessState::Active,
        };
        inner.threads.push(thread.id());

        let res = Rc::new(Self {
            exit_wait: Wait::new(),
            io: IrqSafeSpinLock::new(ProcessIo::new()),
            signal_state: AtomicU32::new(0),
            inner: IrqSafeSpinLock::new(inner),
        });
        debugln!("New kernel process: {:?}", id);
        let prev = PROCESSES.lock().insert(id, res.clone());
        assert!(prev.is_none());
        Ok(res)
    }

    pub fn enqueue(&self) {
        let inner = self.inner.lock();
        for &tid in inner.threads.iter() {
            SCHED.enqueue(tid);
        }
    }

    /// Returns process (if any) to which `pid` refers
    pub fn get(pid: Pid) -> Option<ProcessRef> {
        PROCESSES.lock().get(&pid).cloned()
    }

    /// Sets a pending signal for a process
    pub fn set_signal(&self, signal: Signal) {
        let mut lock = self.inner.lock();
        let main_thread = Thread::get(lock.threads[0]).unwrap();

        // TODO check that `signal` is not a fault signal
        //      it is illegal to call this function with
        //      fault signals

        match main_thread.state() {
            ThreadState::Running => {
                Process::enter_signal_on(lock, main_thread, signal);
            }
            ThreadState::Waiting => {
                // TODO abort whatever the process is waiting for
                todo!()
            }
            ThreadState::Ready => {
                todo!()
            }
            ThreadState::Finished => {
                // TODO report error back
                todo!()
            }
        }
    }

    fn enter_signal_on(mut inner: IrqSafeSpinLockGuard<ProcessInner>, thread: ThreadRef, signal: Signal) {
        let ttbr0 =
            inner.space.as_mut().unwrap().address_phys() | ((inner.id.asid() as usize) << 48);
        drop(inner);
        thread.enter_signal(signal, ttbr0);
    }

    pub fn enter_fault_signal(&self, thread: ThreadRef, signal: Signal) {
        let lock = self.inner.lock();
        Process::enter_signal_on(lock, thread, signal);
    }

    pub fn new_user_thread(&self, entry: usize, stack: usize, arg: usize) -> Result<u32, Errno> {
        let mut lock = self.inner.lock();

        let space_phys = lock.space.as_mut().unwrap().address_phys();
        let ttbr0 = space_phys | ((lock.id.asid() as usize) << 48);

        let thread = Thread::new_user(lock.id, entry, stack, arg, ttbr0)?;
        let tid = thread.id();
        lock.threads.push(tid);
        SCHED.enqueue(tid);

        Ok(tid)
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

        let mut threads = Vec::new();
        let tid = Thread::fork(Some(dst_id), frame, dst_ttbr0)?.id();
        threads.push(tid);

        let dst = Rc::new(Self {
            exit_wait: Wait::new(),
            io: IrqSafeSpinLock::new(src_io.fork()?),
            signal_state: AtomicU32::new(0),
            inner: IrqSafeSpinLock::new(ProcessInner {
                threads,
                exit: None,
                space: Some(dst_space),
                state: ProcessState::Active,
                id: dst_id,
            }),
        });

        debugln!("Process {:?} forked into {:?}", src_inner.id, dst_id);
        assert!(PROCESSES.lock().insert(dst_id, dst).is_none());

        SCHED.enqueue(tid);

        Ok(dst_id)
    }

    // TODO a way to terminate a single thread?
    /// Terminates a process.
    pub fn exit<I: Into<ExitCode>>(status: I) {
        unsafe {
            asm!("msr daifclr, #0xF");
        }
        let status = status.into();
        let thread = Thread::current();
        let process = thread.owner().unwrap();
        let mut lock = process.inner.lock();

        infoln!("Process {:?} is exiting: {:?}", lock.id, status);
        assert!(lock.exit.is_none());
        lock.exit = Some(status);
        lock.state = ProcessState::Finished;

        for &tid in lock.threads.iter() {
            debugln!("Dequeue {:?}", tid);
            Thread::get(tid).unwrap().terminate();
            SCHED.dequeue(tid);
        }
        SCHED.debug();

        if let Some(space) = lock.space.take() {
            unsafe {
                Space::release(space);
                asm!("tlbi aside1, {}", in(reg) ((lock.id.asid() as usize) << 48));
            }
        }

        process.io.lock().handle_exit();

        drop(lock);

        process.exit_wait.wakeup_all();
        SCHED.switch(true);
        panic!("This code should never run");
    }

    pub fn exit_thread(thread: ThreadRef) {
        let switch = {
            let switch = thread.state() == ThreadState::Running;
            let process = thread.owner().unwrap();
            let mut lock = process.inner.lock();
            let tid = thread.id();

            if lock.threads.len() == 1 {
                // TODO call Process::exit instead?
                drop(lock);
                Process::exit(ExitCode::from(0));
                panic!();
            }

            lock.threads.retain(|&e| e != tid);

            thread.terminate();
            SCHED.dequeue(tid);
            debugln!("Thread {} terminated", tid);

            switch
        };

        if switch {
            // TODO retain thread ID in process "finished" list and
            //      drop it when process finishes
            SCHED.switch(true);
            panic!("This code should not run");
        } else {
            // Can drop this thread: it's not running
            todo!();
        }
    }

    fn collect(&self) -> Option<ExitCode> {
        let lock = self.inner.lock();
        if lock.state == ProcessState::Finished {
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

        let proc = Process::current();
        let mut process_lock = proc.inner.lock();

        if process_lock.threads.len() != 1 {
            todo!();
        }

        let thread = Thread::get(process_lock.threads[0]).unwrap();

        if process_lock.id.is_kernel() {
            let mut processes = PROCESSES.lock();
            let old_pid = process_lock.id;
            let new_pid = new_user_pid();
            debugln!("Downgrading process {:?} -> {:?}", old_pid, new_pid);

            let r = processes.remove(&old_pid);
            assert!(r.is_some());
            process_lock.id = new_pid;
            let r = processes.insert(new_pid, proc.clone());
            assert!(r.is_none());
        } else {
            // Invalidate user ASID
            let input = (process_lock.id.asid() as usize) << 48;
            unsafe {
                asm!("tlbi aside1, {}", in(reg) input);
            }
        }

        thread.set_owner(process_lock.id);

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
        process_lock.space = Some(new_space);

        unsafe {
            // TODO drop old context
            let ctx = thread.ctx.get();

            ctx.write(Context::user(
                entry,
                arg,
                new_space_phys | ((process_lock.id.asid() as usize) << 48),
                Self::USTACK_VIRT_TOP,
            ));

            drop(process_lock);

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
