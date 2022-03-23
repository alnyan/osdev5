pub use libsys::abi;
// pub use libsys::calls::*;
pub use libsys::debug;
pub use libsys::error::Errno;
pub use libsys::proc::{self, ExitCode};
pub use libsys::signal::{Signal, SignalDestination};
pub use libsys::stat::{self, AccessMode, FileDescriptor};
pub use libsys::termios;

pub mod calls;
pub use calls::*;

use core::sync::atomic::{AtomicBool, Ordering};

// TODO replace with a proper mutex impl
pub(crate) struct RawMutex {
    inner: AtomicBool,
}

impl RawMutex {
    pub const fn new() -> Self {
        Self {
            inner: AtomicBool::new(false),
        }
    }

    #[inline]
    unsafe fn try_lock(&self) -> bool {
        self.inner
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
    }

    #[inline]
    pub unsafe fn lock(&self) {
        while !self.try_lock() {
            sys_ex_yield();
        }
    }

    #[inline]
    pub unsafe fn release(&self) {
        self.inner.store(false, Ordering::Release);
    }
}
