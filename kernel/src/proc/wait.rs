//! Facilities for process suspension and sleep

use crate::arch::machine;
use crate::dev::timer::TimestampSource;
use crate::proc::{sched::SCHED, Thread, ThreadRef};
use crate::sync::IrqSafeSpinLock;
use alloc::collections::LinkedList;
use core::time::Duration;
use libsys::{error::Errno, proc::Tid, stat::FdSet};

/// Wait channel structure. Contains a queue of processes
/// waiting for some event to happen.
pub struct Wait {
    queue: IrqSafeSpinLock<LinkedList<Tid>>,
    #[allow(dead_code)]
    name: &'static str,
}

/// Status of a (possibly) pending wait
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum WaitStatus {
    /// In progress
    Pending,
    /// Wait was interrupted by a signal
    Interrupted,
    /// Channel reported data available
    Done,
}

struct Timeout {
    tid: Tid,
    deadline: Duration,
}

static TICK_LIST: IrqSafeSpinLock<LinkedList<Timeout>> = IrqSafeSpinLock::new(LinkedList::new());
/// Global wait channel for blocking on select. Gets notified
/// of ANY I/O operations available, so not very efficient.
pub static WAIT_SELECT: Wait = Wait::new("select");

/// Checks for any timed out wait channels and interrupts them
pub fn tick() {
    let time = machine::local_timer().timestamp().unwrap();
    let mut list = TICK_LIST.lock();
    let mut cursor = list.cursor_front_mut();

    while let Some(item) = cursor.current() {
        if time > item.deadline {
            let tid = item.tid;
            cursor.remove_current();
            SCHED.enqueue(tid);
        } else {
            cursor.move_next();
        }
    }
}

/// Suspends current process for given duration
pub fn sleep(timeout: Duration, remaining: &mut Duration) -> Result<(), Errno> {
    // Dummy wait descriptor which will never receive notifications
    static SLEEP_NOTIFY: Wait = Wait::new("sleep");
    let deadline = machine::local_timer().timestamp()? + timeout;
    match SLEEP_NOTIFY.wait(Some(deadline)) {
        Err(Errno::Interrupt) => {
            *remaining = deadline - machine::local_timer().timestamp()?;
            Err(Errno::Interrupt)
        }
        Err(Errno::TimedOut) => Ok(()),
        Ok(_) => panic!("Impossible result"),
        res => res,
    }
}

/// Suspends current process until some file descriptor
/// signals data available
pub fn select(
    thread: ThreadRef,
    mut rfds: Option<&mut FdSet>,
    mut wfds: Option<&mut FdSet>,
    timeout: Option<Duration>,
) -> Result<usize, Errno> {
    if wfds.is_none() && rfds.is_none() {
        todo!();
    }
    let read = rfds.as_deref().map(FdSet::clone);
    let write = wfds.as_deref().map(FdSet::clone);
    if let Some(rfds) = &mut rfds {
        rfds.reset();
    }
    if let Some(wfds) = &mut wfds {
        wfds.reset();
    }

    let deadline = timeout.map(|v| v + machine::local_timer().timestamp().unwrap());
    let proc = thread.owner().unwrap();
    let mut io = proc.io.lock();

    loop {
        if let Some(read) = &read {
            for fd in read.iter() {
                let file = io.file(fd)?;
                if file.borrow().ready(false)? {
                    rfds.as_mut().unwrap().set(fd);
                    return Ok(1);
                }
            }
        }
        if let Some(write) = &write {
            for fd in write.iter() {
                let file = io.file(fd)?;
                if file.borrow().ready(true)? {
                    wfds.as_mut().unwrap().set(fd);
                    return Ok(1);
                }
            }
        }

        // Suspend
        match WAIT_SELECT.wait(deadline) {
            Err(Errno::TimedOut) => return Ok(0),
            Err(e) => return Err(e),
            Ok(_) => {}
        }
    }
}

impl Wait {
    /// Constructs a new wait channel
    pub const fn new(name: &'static str) -> Self {
        Self {
            queue: IrqSafeSpinLock::new(LinkedList::new()),
            name,
        }
    }

    /// Interrupt wait pending on the channel
    pub fn abort(&self, tid: Tid, enqueue: bool) {
        let mut queue = self.queue.lock();
        let mut tick_lock = TICK_LIST.lock();
        let mut cursor = tick_lock.cursor_front_mut();
        while let Some(item) = cursor.current() {
            if tid == item.tid {
                cursor.remove_current();
                break;
            } else {
                cursor.move_next();
            }
        }

        let mut cursor = queue.cursor_front_mut();
        while let Some(item) = cursor.current() {
            if tid == *item {
                cursor.remove_current();
                let thread = Thread::get(tid).unwrap();
                thread.set_wait_status(WaitStatus::Interrupted);
                if enqueue {
                    SCHED.enqueue(tid);
                }
                break;
            } else {
                cursor.move_next();
            }
        }
    }

    fn wakeup_some(&self, mut limit: usize) -> usize {
        // No IRQs will arrive now == safe to manipulate tick list
        let mut queue = self.queue.lock();
        let mut count = 0;
        while limit != 0 && !queue.is_empty() {
            let tid = queue.pop_front();
            if let Some(tid) = tid {
                let mut tick_lock = TICK_LIST.lock();
                let mut cursor = tick_lock.cursor_front_mut();
                while let Some(item) = cursor.current() {
                    if tid == item.tid {
                        cursor.remove_current();
                        break;
                    } else {
                        cursor.move_next();
                    }
                }
                drop(tick_lock);

                Thread::get(tid).unwrap().set_wait_status(WaitStatus::Done);
                SCHED.enqueue(tid);
            }

            limit -= 1;
            count += 1;
        }
        count
    }

    /// Notifies all processes waiting for this event
    pub fn wakeup_all(&self) {
        self.wakeup_some(usize::MAX);
    }

    /// Notifies a single process waiting for this event
    pub fn wakeup_one(&self) {
        self.wakeup_some(1);
    }

    /// Suspends current process until event is signalled or
    /// (optional) deadline is reached
    pub fn wait(&self, deadline: Option<Duration>) -> Result<(), Errno> {
        let thread = Thread::current();
        //let deadline = timeout.map(|t| machine::local_timer().timestamp().unwrap() + t);
        let mut queue_lock = self.queue.lock();

        queue_lock.push_back(thread.id());
        thread.setup_wait(self);

        if let Some(deadline) = deadline {
            TICK_LIST.lock().push_back(Timeout {
                tid: thread.id(),
                deadline,
            });
        }

        loop {
            match thread.wait_status() {
                WaitStatus::Pending => {}
                WaitStatus::Done => {
                    return Ok(());
                }
                WaitStatus::Interrupted => {
                    return Err(Errno::Interrupt);
                }
            };

            drop(queue_lock);
            thread.enter_wait();
            queue_lock = self.queue.lock();

            if let Some(deadline) = deadline {
                if machine::local_timer().timestamp()? > deadline {
                    let mut cursor = queue_lock.cursor_front_mut();

                    while let Some(&mut item) = cursor.current() {
                        if thread.id() == item {
                            cursor.remove_current();
                            break;
                        } else {
                            cursor.move_next();
                        }
                    }

                    return Err(Errno::TimedOut);
                }
            }
        }
    }
}
