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
    todo!();
}

/// Suspends current process for given duration
pub fn sleep(timeout: Duration, remaining: &mut Duration) -> Result<(), Errno> {
    todo!()
}

/// Suspends current process until some file descriptor
/// signals data available
pub fn select(
    thread: ThreadRef,
    mut rfds: Option<&mut FdSet>,
    mut wfds: Option<&mut FdSet>,
    timeout: Option<Duration>,
) -> Result<usize, Errno> {
    todo!();
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
        todo!();
    }

    fn wakeup_some(&self, mut limit: usize) -> usize {
        todo!();
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
        todo!();
    }
}
