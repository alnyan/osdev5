//! Process data and control
use crate::arch::aarch64::exception::ExceptionFrame;
use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::{MapAttributes, Space},
};
use crate::proc::{
    wait::Wait, Context, ProcessIo, Thread, ThreadRef, ThreadState, PROCESSES, sched,
};
use crate::sync::IrqSafeSpinLock;
use alloc::{rc::Rc, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};
use libsys::{
    error::Errno,
    mem::memcpy,
    proc::{ExitCode, Pid},
    signal::Signal,
    ProgramArgs,
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
    pgid: Pid,
    ppid: Option<Pid>,
    sid: Pid,
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
        F: FnOnce(&mut Space) -> R,
    {
        f(self.inner.lock().space.as_mut().unwrap())
    }

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
            sched::enqueue(tid);
        }
    }

    /// Returns process (if any) to which `pid` refers
    pub fn get(pid: Pid) -> Option<ProcessRef> {
        PROCESSES.lock().get(&pid).cloned()
    }

// <<<<<<< HEAD
    // /// Schedules an initial thread for execution
    // ///
    // /// # Safety
    // ///
    // /// Unsafe: only allowed to be called once, repeated calls
    // ///         will generate undefined behavior
    // pub unsafe fn enter(cpu: u32, proc: ProcessRef) -> ! {
    //     // FIXME use some global lock to guarantee atomicity of thread entry?
    //     proc.inner.lock().state = State::Running;
    //     proc.cpu.store(cpu, Ordering::SeqCst);
    //     let ctx = proc.ctx.get();

    //     // I don't think this is bad: process can't be dropped fully unless
    //     // it's been reaped (and this function won't run for such process)
    //     // drop(proc);
    //     (&mut *ctx).enter()
    // }
// =======
    /// Sets a pending signal for a process
    pub fn set_signal(&self, signal: Signal) {
        todo!();
        // let mut lock = self.inner.lock();
        // let ttbr0 = lock.space.as_mut().unwrap().address_phys() | ((lock.id.asid() as usize) << 48);
        // let main_thread = Thread::get(lock.threads[0]).unwrap();
        // drop(lock);

        // // TODO check that `signal` is not a fault signal
        // //      it is illegal to call this function with
        // //      fault signals
        // match main_thread.state() {
        //     ThreadState::Running => {
        //         main_thread.enter_signal(signal, ttbr0);
        //     }
        //     ThreadState::Waiting => {
        //         main_thread.clone().setup_signal(signal, ttbr0);
        //         main_thread.interrupt_wait(true);
        //     }
        //     ThreadState::Ready => {
        //         main_thread.clone().setup_signal(signal, ttbr0);
        //         main_thread.interrupt_wait(false);
        //     }
        //     ThreadState::Finished => {
        //         // TODO report error back
        //         todo!()
        //     }
        // }
    }

    /// Immediately delivers a signal to requested thread
    pub fn enter_fault_signal(&self, thread: ThreadRef, signal: Signal) {
        todo!();
        // let mut lock = self.inner.lock();
        // let ttbr0 = lock.space.as_mut().unwrap().address_phys() | ((lock.id.asid() as usize) << 48);
        // thread.enter_signal(signal, ttbr0);
    }

    // /// Schedules a next thread for execution
    // ///
    // /// # Safety
    // ///
    // /// Unsafe:
    // ///
    // /// * Does not ensure src and dst threads are not the same thread
    // /// * Does not ensure src is actually current context
    // pub unsafe fn switch(cpu: u32, src: ProcessRef, dst: ProcessRef, discard: bool) {
    //     {
    //         let mut src_lock = src.inner.lock();
    //         let mut dst_lock = dst.inner.lock();

    //         if !discard {
    //             assert_eq!(src_lock.state, State::Running);
    //             src_lock.state = State::Ready;
    //         }
    //         assert!(dst_lock.state == State::Ready || dst_lock.state == State::Waiting);
    //         dst_lock.state = State::Running;

    //         src.cpu.store(Self::CPU_NONE, Ordering::SeqCst);
    //         dst.cpu.store(cpu, Ordering::SeqCst);
    //     }

    //     let src_ctx = src.ctx.get();
    //     let dst_ctx = dst.ctx.get();

    //     // See "drop" note in Process::enter()
    //     // drop(src);
    //     // drop(dst);

    //     (&mut *src_ctx).switch(&mut *dst_ctx);
    // }

    // /// Suspends current process with a "waiting" status
    // pub fn enter_wait(&self) {
    //     let drop = {
    //         let mut lock = self.inner.lock();
    //         let drop = lock.state == State::Running;
    //         lock.state = State::Waiting;
    //         sched::dequeue(lock.id);
    //         // SCHED.dequeue(lock.id);
    //         drop
    //     };
    //     if drop {
    //         sched::switch(true);
    //         // todo!();
    //         // SCHED.switch(true);
    //     }
    // }

    /// Crates a new thread in the process
    pub fn new_user_thread(&self, entry: usize, stack: usize, arg: usize) -> Result<u32, Errno> {
        let mut lock = self.inner.lock();

        let space_phys = lock.space.as_mut().unwrap().address_phys();
        let ttbr0 = space_phys | ((lock.id.asid() as usize) << 48);

        let thread = Thread::new_user(lock.id, entry, stack, arg, ttbr0)?;
        let tid = thread.id();
        lock.threads.push(tid);
        sched::enqueue(tid);

        Ok(tid)
    }

    // /// Creates a new kernel process
    // pub fn new_kernel(entry: extern "C" fn(usize) -> !, arg: usize) -> Result<ProcessRef, Errno> {
    //     let id = Pid::new_kernel();
    //     let res = Rc::new(Self {
    //         ctx: UnsafeCell::new(Context::kernel(entry as usize, arg)),
    //         io: IrqSafeSpinLock::new(ProcessIo::new()),
    //         exit_wait: Wait::new(),
    //         inner: IrqSafeSpinLock::new(ProcessInner {
    //             id,
    //             exit: None,
    //             space: None,
    //             wait_flag: false,
    //             state: State::Ready,
    //         }),
    //         cpu: AtomicU32::new(Self::CPU_NONE),
    //     });
    //     debugln!("New kernel process: {}", id);
    //     assert!(PROCESSES.lock().insert(id, res.clone()).is_none());
    //     Ok(res)
    // }

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

        sched::enqueue(tid);
        // SCHED.enqueue(dst_id);

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
            Thread::get(tid).unwrap().terminate(status);
            sched::dequeue(tid);
            // SCHED.dequeue(tid);
        }

        if let Some(space) = lock.space.take() {
            unsafe {
                Space::release(space);
                asm!("tlbi aside1, {}", in(reg) ((lock.id.asid() as usize) << 48));
            }
        }

        // TODO when exiting from signal handler interrupting an IO operation
        //      deadlock is achieved
        self.io.lock().handle_exit();

        drop(lock);

        self.exit_wait.wakeup_all();

        if is_running {
            sched::switch(true);
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
            todo!();
            // SCHED.dequeue(tid);
            debugln!("Thread {} terminated", tid);

            switch
        };

        if switch {
            // TODO retain thread ID in process "finished" list and
            //      drop it when process finishes
            // SCHED.switch(true);
            todo!();
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

    fn write_paged<T>(space: &mut Space, dst: usize, src: T) -> Result<(), Errno> {
        let size = core::mem::size_of::<T>();
        if (size + (dst % 4096)) > 4096 {
            todo!("Object crossed page boundary");
        }

        let page_virt = dst & !4095;
        let page_phys = if let Ok(phys) = space.translate(dst) {
            phys
        } else {
            let page = phys::alloc_page(PageUsage::UserPrivate)?;
            let flags = MapAttributes::SH_OUTER
                | MapAttributes::NOT_GLOBAL
                | MapAttributes::UXN
                | MapAttributes::PXN
                | MapAttributes::AP_BOTH_READONLY;
            space.map(page_virt, page, flags)?;
            page
        };

        unsafe {
            core::ptr::write((mem::virtualize(page_phys) + (dst % 4096)) as *mut T, src);
        }
        Ok(())
    }

    fn write_paged_bytes(space: &mut Space, dst: usize, src: &[u8]) -> Result<(), Errno> {
        if (src.len() + (dst % 4096)) > 4096 {
            todo!("Object crossed page boundary");
        }
        let page_virt = dst & !4095;
        let page_phys = if let Ok(phys) = space.translate(dst) {
            phys
        } else {
            let page = phys::alloc_page(PageUsage::UserPrivate)?;
            let flags = MapAttributes::SH_OUTER
                | MapAttributes::NOT_GLOBAL
                | MapAttributes::UXN
                | MapAttributes::PXN
                | MapAttributes::AP_BOTH_READONLY;
            space.map(page_virt, page, flags)?;
            page
        };

        unsafe {
            memcpy(
                (mem::virtualize(page_phys) + (dst % 4096)) as *mut u8,
                src.as_ptr(),
                src.len(),
            );
        }
        Ok(())
    }

    fn store_arguments(space: &mut Space, argv: &[&str]) -> Result<usize, Errno> {
        let mut offset = 0usize;
        // TODO vmalloc?
        let base = 0x60000000;

        // 1. Store program argument string bytes
        for arg in argv.iter() {
            Self::write_paged_bytes(space, base + offset, arg.as_bytes())?;
            offset += arg.len();
        }
        // Align
        offset = (offset + 15) & !15;
        let argv_offset = offset;

        // 2. Store arg pointers
        let mut data_offset = 0usize;
        for arg in argv.iter() {
            // XXX this is really unsafe and I am not really sure ABI will stay like this XXX
            Self::write_paged(space, base + offset + 0, base + data_offset)?;
            Self::write_paged(space, base + offset + 8, arg.len())?;
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
        Self::write_paged(space, base + offset, data)?;

        Ok(base + offset)
    }

   /// Loads a new program into current process address space
   pub fn execve<F: FnOnce(&mut Space) -> Result<usize, Errno>>(
       loader: F,
       argv: &[&str],
   ) -> Result<(), Errno> {
       unsafe {
           // Run with interrupts disabled
           asm!("msr daifset, #2");
       }

// <<<<<<< HEAD
//         let proc = sched::current_process();
//         let mut lock = proc.inner.lock();
//         if lock.id.is_kernel() {
//             let mut proc_lock = PROCESSES.lock();
//             let old_pid = lock.id;
//             assert!(
//                 proc_lock.remove(&old_pid).is_some(),
//                 "Failed to downgrade kernel process (remove kernel pid)"
//             );
//             lock.id = Pid::new_user();
//             debugln!(
//                 "Process downgrades from kernel to user: {} -> {}",
//                 old_pid,
//                 lock.id
//             );
//             assert!(proc_lock.insert(lock.id, proc.clone()).is_none());
//             unsafe {
//                 use crate::arch::platform::cpu::Cpu;
//                 Cpu::get().scheduler().hack_current_pid(lock.id);
//             }
//         } else {
//             // Invalidate user ASID
//             let input = (lock.id.asid() as usize) << 48;
//             unsafe {
//                 asm!("tlbi aside1, {}", in(reg) input);
//             }
// =======
        let proc = Process::current();
        let mut process_lock = proc.inner.lock();

        if process_lock.threads.len() != 1 {
            todo!();
// >>>>>>> feat/thread
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
        let arg = Self::store_arguments(new_space, argv)?;

        debugln!("Will now enter at {:#x}", entry);
        // TODO drop old address space
        process_lock.space = Some(new_space);

        unsafe {
            // TODO drop old context
            let ctx = thread.ctx.get();
            let asid = (process_lock.id.asid() as usize) << 48;
            asm!("tlbi aside1, {}", in(reg) asid);

            ctx.write(Context::user(
                entry,
                arg,
                new_space_phys | asid,
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
