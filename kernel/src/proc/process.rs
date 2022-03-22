//! Process data and control
use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::{write_paged, write_paged_bytes, table::{MapAttributes, Space, SpaceImpl}},
};
use crate::proc::{
    wait::Wait, Context, ProcessIo, Thread, ThreadRef, ThreadState, Tid, PROCESSES, SCHED,
};
use crate::arch::{intrin, platform::ForkFrame};
use crate::sync::{IrqSafeSpinLock, IrqSafeSpinLockGuard};
use alloc::{rc::Rc, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};
use libsys::{
    error::Errno,
    proc::{ExitCode, Pid},
    signal::Signal,
    ProgramArgs,
};
use core::arch::asm;

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
    space: Option<&'static mut SpaceImpl>,
    state: ProcessState,
    id: Pid,
    pgid: Pid,
    ppid: Option<Pid>,
    sid: Pid,
    exit: Option<ExitCode>,
    threads: Vec<Tid>,
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
    const USTACK_PAGES: usize = 8;

    /// Returns the process ID
    #[inline]
    pub fn id(&self) -> Pid {
        self.inner.lock().id
    }

    /// Returns the process session ID
    #[inline]
    pub fn sid(&self) -> Pid {
        self.inner.lock().sid
    }

    /// Returns parent's [Pid]
    #[inline]
    pub fn pgid(&self) -> Pid {
        self.inner.lock().pgid
    }

    /// Returns parent's [Pid]
    #[inline]
    pub fn ppid(&self) -> Option<Pid> {
        self.inner.lock().ppid
    }

    /// Sets a new group id for the process
    pub fn set_pgid(&self, pgid: Pid) {
        self.inner.lock().pgid = pgid;
    }

    /// Sets a new session id for the process
    pub fn set_sid(&self, sid: Pid) {
        self.inner.lock().sid = sid;
    }

    /// Returns [Rc]-reference to current process
    #[inline]
    pub fn current() -> ProcessRef {
        Thread::current().owner().unwrap()
    }

    /// Executes a closure performing manipulations on the process address space
    #[inline]
    pub fn manipulate_space<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut SpaceImpl) -> R,
    {
        f(self.inner.lock().space.as_mut().unwrap())
    }

    /// Handles all pending signals (when returning from aborted syscall)
    pub fn handle_pending_signals(&self) {
        let mut lock = self.inner.lock();
        let table = Self::space_phys(&mut lock);
        let main_thread = Thread::get(lock.threads[0]).unwrap();
        drop(lock);

        loop {
            let state = self.signal_state.load(Ordering::Acquire);
            if let Some(signal) = Self::find1(state).map(|e| Signal::try_from(e as u32).unwrap()) {
                self.signal_state.fetch_and(!(1 << (signal as u32)), Ordering::Release);
                main_thread.clone().enter_signal(signal, table);
            } else {
                break;
            }
        }
    }

    pub fn set_signal(&self, signal: Signal) {
        let mut lock = self.inner.lock();
        let table = Self::space_phys(&mut lock);
        let main_thread = Thread::get(lock.threads[0]).unwrap();
        drop(lock);

        // TODO check that `signal` is not a fault signal
        //      it is illegal to call this function with
        //      fault signals

        match main_thread.state() {
            ThreadState::Running => {
                main_thread.enter_signal(signal, table);
            }
            ThreadState::Waiting => {
                self.signal_state.fetch_or(1 << (signal as u32), Ordering::Release);
                main_thread.interrupt_wait(true);
            }
            ThreadState::Ready => {
                main_thread.clone().setup_signal(signal, table);
                main_thread.interrupt_wait(false);
            }
            ThreadState::Finished => {
                // TODO report error back
                todo!()
            }
        }
    }

    // /// Immediately delivers a signal to requested thread
    // pub fn enter_fault_signal(&self, thread: ThreadRef, signal: Signal) {
    //     let mut lock = self.inner.lock();
    //     let table = Self::space_phys(&lock);
    //     drop(lock);
    //     thread.enter_signal(signal, table);
    // }

    /// Creates a new kernel process
    pub fn new_kernel(entry: extern "C" fn(usize) -> !, arg: usize) -> Result<ProcessRef, Errno> {
        let id = new_kernel_pid();
        let thread = Thread::new_kernel(Some(id), entry, arg)?;
        let mut inner = ProcessInner {
            threads: Vec::new(),
            id,
            pgid: id,
            ppid: None,
            sid: id,
            exit: None,
            space: None,
            state: ProcessState::Active,
        };
        inner.threads.push(thread.id());

        let res = Rc::new(Self {
            exit_wait: Wait::new("process_exit"),
            io: IrqSafeSpinLock::new(ProcessIo::new()),
            signal_state: AtomicU32::new(0),
            inner: IrqSafeSpinLock::new(inner),
        });
        debugln!("New kernel process: {:?}", id);
        let prev = PROCESSES.lock().insert(id, res.clone());
        assert!(prev.is_none());
        Ok(res)
    }

    /// Adds all of the process threads to scheduler queue
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

    fn find1(a: u32) -> Option<usize> {
        for i in 0..32 {
            if a & (1 << i) != 0 {
                return Some(i);
            }
        }
        None
    }

    fn space_phys(lock: &mut IrqSafeSpinLockGuard<ProcessInner>) -> usize {
        lock.space.as_mut().unwrap().address_phys() | ((lock.id.asid() as usize) << 48)
    }

    /// Creates a "fork" of the process, cloning its address space and
    /// resources
    pub fn fork(&self, frame: &mut ForkFrame) -> Result<Pid, Errno> {
        let src_io = self.io.lock();
        let mut src_inner = self.inner.lock();

        let dst_id = new_user_pid();
        let dst_space = src_inner.space.as_mut().unwrap().fork()?;

        let dst_space_phys = (dst_space as *mut _ as usize) - mem::KERNEL_OFFSET;
        // let dst_ttbr0 = dst_space_phys | ((dst_id.asid() as usize) << 48);

        let mut threads = Vec::new();
        let tid = Thread::fork(Some(dst_id), frame, dst_space_phys)?.id();
        threads.push(tid);

        let dst = Rc::new(Self {
            exit_wait: Wait::new("process_exit"),
            io: IrqSafeSpinLock::new(src_io.fork()?),
            signal_state: AtomicU32::new(0),
            inner: IrqSafeSpinLock::new(ProcessInner {
                threads,
                exit: None,
                space: Some(dst_space),
                state: ProcessState::Active,
                id: dst_id,
                pgid: src_inner.pgid,
                ppid: Some(src_inner.id),
                sid: src_inner.sid,
            }),
        });

        debugln!("Process {:?} forked into {:?}", src_inner.id, dst_id);
        assert!(PROCESSES.lock().insert(dst_id, dst).is_none());

        SCHED.enqueue(tid);

        Ok(dst_id)
    }

    /// Terminates a process.
    pub fn exit(self: ProcessRef, status: ExitCode) {
        let thread = Thread::current();
        let mut lock = self.inner.lock();
        let is_running = thread.owner_id().map(|e| e == lock.id).unwrap_or(false);

        infoln!("Process {:?} is exiting: {:?}", lock.id, status);
        assert!(lock.exit.is_none());
        lock.exit = Some(status);
        lock.state = ProcessState::Finished;

        for &tid in lock.threads.iter() {
            let thread = Thread::get(tid).unwrap();
            if thread.state() == ThreadState::Waiting {
                todo!()
            }
            thread.terminate(status);
            SCHED.dequeue(tid);
        }

        if let Some(space) = lock.space.take() {
            unsafe {
                SpaceImpl::release(space);
                // Process::invalidate_asid((lock.id.asid() as usize) << 48);
            }
        }

        // TODO when exiting from signal handler interrupting an IO operation
        //      deadlock is achieved
        self.io.lock().handle_exit();

        drop(lock);

        self.exit_wait.wakeup_all();

        if is_running {
            SCHED.switch(true);
            panic!("This code should never run");
        }
    }

    /// Terminates a thread of the process. If the thread is the only
    /// one remaining, process itself is exited (see [Process::exit])
    pub fn exit_thread(thread: ThreadRef, status: ExitCode) {
        let switch = {
            let switch = thread.state() == ThreadState::Running;
            let process = thread.owner().unwrap();
            let mut lock = process.inner.lock();
            let tid = thread.id();

            if lock.threads.len() == 1 {
                // TODO call Process::exit instead?
                drop(lock);
                process.exit(status);
                return;
            }

            lock.threads.retain(|&e| e != tid);

            thread.terminate(status);
            SCHED.dequeue(tid);
            debugln!("Thread {:?} terminated", tid);

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
                return Ok(r);
            }

            proc.exit_wait.wait(None)?;
        }
    }

    fn store_arguments(space: &mut SpaceImpl, argv: &[&str]) -> Result<usize, Errno> {
        let mut offset = 0usize;
        // TODO vmalloc?
        let base = 0x60000000;

        // 1. Store program argument string bytes
        for arg in argv.iter() {
            unsafe {
                write_paged_bytes(space, base + offset, arg.as_bytes())?;
            }
            offset += arg.len();
        }
        // Align
        offset = (offset + 15) & !15;
        let argv_offset = offset;

        // 2. Store arg pointers
        let mut data_offset = 0usize;
        for arg in argv.iter() {
            // XXX this is really unsafe and I am not really sure ABI will stay like this XXX
            unsafe {
                write_paged(space, base + offset, base + data_offset)?;
                write_paged(space, base + offset + 8, arg.len())?;
            }
            offset += 16;
            data_offset += arg.len();
        }

        // 3. Store ProgramArgs
        let data = ProgramArgs {
            argc: argv.len(),
            argv: base + argv_offset,
            storage: base,
            size: offset + core::mem::size_of::<ProgramArgs>(),
        };
        unsafe {
            write_paged(space, base + offset, data)?;
        }

        Ok(base + offset)
    }

    /// Returns the process's address space ID
    pub fn asid(&self) -> usize {
        (self.id().asid() as usize) << 48
    }

    /// Loads a new program into current process address space
    pub fn execve<F: FnOnce(&mut SpaceImpl) -> Result<usize, Errno>>(
        loader: F,
        argv: &[&str],
    ) -> Result<(), Errno> {
        unsafe {
            // Run with interrupts disabled
            intrin::irq_disable();
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
            process_lock.pgid = new_pid;
            process_lock.sid = new_pid;
            let r = processes.insert(new_pid, proc.clone());
            assert!(r.is_none());
        }

        thread.set_owner(process_lock.id);

        proc.io.lock().handle_cloexec();

        let new_space = SpaceImpl::alloc_empty()?;
        let new_space_phys = (new_space as *mut _ as usize) - mem::KERNEL_OFFSET;

        let ustack_virt_bottom = Self::USTACK_VIRT_TOP - Self::USTACK_PAGES * mem::PAGE_SIZE;
        for i in 0..Self::USTACK_PAGES {
            let page = phys::alloc_page(PageUsage::UserPrivate).unwrap();
            let flags = MapAttributes::SHARE_OUTER
                | MapAttributes::USER_READ
                | MapAttributes::USER_WRITE;
            new_space
                .map(ustack_virt_bottom + i * mem::PAGE_SIZE, page, flags)
                .unwrap();
        }

        let entry = loader(new_space)?;
        let arg = Self::store_arguments(new_space, argv)?;

        // TODO drop old address space
        process_lock.space = Some(new_space);

        unsafe {
            // TODO drop old context
            let ctx = thread.ctx.get();
            let asid = (process_lock.id.asid() as usize) << 48;
            // Process::invalidate_asid(asid);

            ctx.write(Context::user(
                entry,
                arg,
                new_space_phys | asid,
                Self::USTACK_VIRT_TOP - 8,
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
