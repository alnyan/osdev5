pub use libsys::signal::{Signal, SignalDestination};
pub use libsys::proc::ExitCode;
pub use libsys::termios;
pub use libsys::abi;
pub use libsys::calls::*;
pub use libsys::stat::{self, AccessMode, FileDescriptor};
pub use libsys::error::Errno;

use core::sync::atomic::{Ordering, AtomicBool};

// TODO replace with a proper mutex impl
pub(crate) struct RawMutex {
    inner: AtomicBool
}

impl RawMutex {
    pub const fn new() -> Self {
        Self { inner: AtomicBool::new(false) }
    }

    #[inline]
    unsafe fn try_lock(&self) -> bool {
        self.inner.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_ok()
    }

    #[inline]
    unsafe fn is_locked(&self) -> bool {
        self.inner.load(Ordering::Acquire)
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
