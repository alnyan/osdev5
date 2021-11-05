//! Facilities for process suspension and sleep

use crate::arch::machine;
use crate::dev::timer::TimestampSource;
use crate::proc::{self, sched::SCHED, Pid, Process};
use crate::sync::IrqSafeSpinLock;
use alloc::collections::LinkedList;
use core::time::Duration;
use error::Errno;

/// Wait channel structure. Contains a queue of processes
/// waiting for some event to happen.
pub struct Wait {
    queue: IrqSafeSpinLock<LinkedList<Pid>>,
}

struct Timeout {
    pid: Pid,
    deadline: Duration,
}

static TICK_LIST: IrqSafeSpinLock<LinkedList<Timeout>> = IrqSafeSpinLock::new(LinkedList::new());

/// Checks for any timed out wait channels and interrupts them
pub fn tick() {
    let time = machine::local_timer().timestamp().unwrap();
    let mut list = TICK_LIST.lock();
    let mut cursor = list.cursor_front_mut();

    while let Some(item) = cursor.current() {
        if time > item.deadline {
            let pid = item.pid;
            cursor.remove_current();
            SCHED.enqueue(pid);
        } else {
            cursor.move_next();
        }
    }
}

/// Suspends current process for given duration
pub fn sleep(timeout: Duration, remaining: &mut Duration) -> Result<(), Errno> {
    // Dummy wait descriptor which will never receive notifications
    static SLEEP_NOTIFY: Wait = Wait::new();
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

impl Wait {
    /// Constructs a new wait channel
    pub const fn new() -> Self {
        Self {
            queue: IrqSafeSpinLock::new(LinkedList::new()),
        }
    }

    fn wakeup_some(&self, mut limit: usize) -> usize {
        // No IRQs will arrive now == safe to manipulate tick list
        let mut queue = self.queue.lock();
        let mut count = 0;
        while limit != 0 && !queue.is_empty() {
            let pid = queue.pop_front();
            if let Some(pid) = pid {
                let mut tick_lock = TICK_LIST.lock();
                let mut cursor = tick_lock.cursor_front_mut();
                while let Some(item) = cursor.current() {
                    if pid == item.pid {
                        cursor.remove_current();
                        break;
                    } else {
                        cursor.move_next();
                    }
                }
                drop(tick_lock);

                proc::process(pid).set_wait_flag(false);
                SCHED.enqueue(pid);
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
        let proc = Process::current();
        //let deadline = timeout.map(|t| machine::local_timer().timestamp().unwrap() + t);
        let mut queue_lock = self.queue.lock();

        queue_lock.push_back(proc.id());
        proc.set_wait_flag(true);
        if let Some(deadline) = deadline {
            TICK_LIST.lock().push_back(Timeout {
                pid: proc.id(),
                deadline,
            });
        }

        loop {
            if !proc.wait_flag() {
                return Ok(());
            }

            drop(queue_lock);
            proc.enter_wait();
            queue_lock = self.queue.lock();

            if let Some(deadline) = deadline {
                if machine::local_timer().timestamp()? > deadline {
                    let mut cursor = queue_lock.cursor_front_mut();

                    while let Some(&mut item) = cursor.current() {
                        if proc.id() == item {
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
