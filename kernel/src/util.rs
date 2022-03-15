//! Various utilities used by the kernel

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

/// Wrapper structure to guarantee single initialization
/// of a value
pub struct InitOnce<T> {
    state: AtomicBool,
    inner: UnsafeCell<MaybeUninit<T>>,
}

impl<T> InitOnce<T> {
    /// Constructs a new instance of [InitOnce<T>]
    pub const fn new() -> Self {
        Self {
            state: AtomicBool::new(false),
            inner: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Returns `true` if this [InitOnce<T>] can be used
    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.state.load(Ordering::Acquire)
    }

    /// Returns the initialized value. Will panic if the value has not
    /// yet been initialized.
    #[allow(clippy::mut_from_ref)]
    #[track_caller]
    pub fn get(&self) -> &mut T {
        assert!(
            self.is_initialized(),
            "Access to uninitialized InitOnce<T>: {:?}",
            core::panic::Location::caller()
        );
        unsafe { (*self.inner.get()).assume_init_mut() }
    }

    /// Initializes the storage with `value`. Will panic if the storage has
    /// already been initialized.
    pub fn init(&self, value: T) {
        assert!(
            self.state
                .compare_exchange_weak(false, true, Ordering::Release, Ordering::Relaxed)
                .is_ok(),
            "Double-initialization of InitOnce<T>"
        );

        unsafe {
            (*self.inner.get()).write(value);
        }
    }
}

unsafe impl<T> Sync for InitOnce<T> {}

///
#[macro_export]
macro_rules! block {
    ($cond:expr, $timeout:expr, $failure:expr) => {{
        use $crate::dev::timer::TimestampSource;
        let __deadline = $crate::arch::machine::local_timer().timestamp().unwrap()
            + core::time::Duration::from_micros($timeout);
        loop {
            if $cond {
                break;
            }
            if $crate::arch::machine::local_timer().timestamp().unwrap() > __deadline {
                $failure
            }
        }
    }};

    ($cond:expr, $timeout:expr) => {
        crate::block!($cond, $timeout, return Err(libsys::error::Errno::TimedOut))
    };
}
