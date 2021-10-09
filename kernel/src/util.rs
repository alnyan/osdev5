#![allow(missing_docs)]

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

pub struct InitOnce<T> {
    state: AtomicBool,
    inner: UnsafeCell<MaybeUninit<T>>,
}

impl<T> InitOnce<T> {
    pub const fn new() -> Self {
        Self {
            state: AtomicBool::new(false),
            inner: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    #[inline(always)]
    pub fn is_initialized(&self) -> bool {
        self.state.load(Ordering::Acquire)
    }

    pub fn get(&self) -> &mut T {
        assert!(self.is_initialized(), "Access to uninitialized InitOnce<T>");
        unsafe { (*self.inner.get()).assume_init_mut() }
    }

    pub fn init(&self, value: T) {
        assert!(
            self.state
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok(),
            "Double-initialization of InitOnce<T>"
        );

        unsafe {
            (*self.inner.get()).write(value);
        }
    }
}

unsafe impl<T> Sync for InitOnce<T> {}
