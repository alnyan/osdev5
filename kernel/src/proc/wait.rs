use crate::arch::machine;
use crate::dev::timer::TimestampSource;
use crate::proc::{self, sched::SCHED, Pid, Process};
use crate::sync::IrqSafeSpinLock;
use alloc::collections::LinkedList;
use core::time::Duration;
use error::Errno;

pub struct Wait {
    queue: IrqSafeSpinLock<LinkedList<Pid>>,
}

pub struct Timeout {
    pid: Pid,
    deadline: Duration,
}

static TICK_LIST: IrqSafeSpinLock<LinkedList<Timeout>> = IrqSafeSpinLock::new(LinkedList::new());

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

pub fn sleep(timeout: Duration) {
    // Dummy wait descriptor which will never receive notifications
    static SLEEP_NOTIFY: Wait = Wait::new();
    SLEEP_NOTIFY.sleep_on(Some(timeout)).ok();
}

impl Wait {
    pub const fn new() -> Self {
        Self {
            queue: IrqSafeSpinLock::new(LinkedList::new()),
        }
    }

    pub fn wakeup_all(&self) {
        todo!()
    }

    pub fn wakeup_one(&self) {
        // No IRQs will arrive now == safe to manipulate tick list
        let mut tick_lock = TICK_LIST.lock();
        let pid = self.queue.lock().pop_front();
        if let Some(pid) = pid {
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
    }

    pub fn sleep_on(&self, timeout: Option<Duration>) -> Result<(), Errno> {
        let proc = Process::current();
        let deadline = timeout.map(|t| machine::local_timer().timestamp().unwrap() + t);
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
