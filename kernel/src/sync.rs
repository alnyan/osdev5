//! Synchronization facilities module

use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// Dummy lock implementation, does not do any locking.
///
/// Only safe to use before I implement context switching or
/// interrupts are enabled.
#[repr(transparent)]
pub struct NullLock<T: ?Sized> {
    value: UnsafeCell<T>,
}

/// Dummy lock guard for [NullLock].
#[repr(transparent)]
pub struct NullLockGuard<'a, T: ?Sized> {
    value: &'a mut T,
}

impl<T> NullLock<T> {
    /// Constructs a new instance of the lock, wrapping `value`
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Returns [NullLockGuard] for this lock
    #[inline(always)]
    pub fn lock(&self) -> NullLockGuard<T> {
        NullLockGuard {
            value: unsafe { &mut *self.value.get() },
        }
    }
}

impl<T: ?Sized> Deref for NullLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T: ?Sized> DerefMut for NullLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

unsafe impl<T: ?Sized> Sync for NullLock<T> {}

pub use NullLock as Spin;
