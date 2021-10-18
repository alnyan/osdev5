//! Synchronization facilities module

use crate::arch::platform::{irq_mask_save, irq_restore};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

/// Same as [NullLock], but ensures IRQs are disabled while
/// the lock is held
pub struct IrqSafeNullLock<T: ?Sized> {
    value: UnsafeCell<T>,
}

/// Same as [NullLockGuard], but reverts IRQ mask back to normal
/// when dropped
pub struct IrqSafeNullLockGuard<'a, T: ?Sized> {
    value: &'a mut T,
    irq_state: u64,
}

impl<T> IrqSafeNullLock<T> {
    /// Constructs a new instance of the lock, wrapping `value`
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Returns [IrqSafeNullLockGuard] for this lock
    #[inline]
    pub fn lock(&self) -> IrqSafeNullLockGuard<T> {
        unsafe {
            IrqSafeNullLockGuard {
                value: &mut *self.value.get(),
                irq_state: irq_mask_save(),
            }
        }
    }
}

impl<T: ?Sized> Deref for IrqSafeNullLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T: ?Sized> DerefMut for IrqSafeNullLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

impl<T: ?Sized> Drop for IrqSafeNullLockGuard<'_, T> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            irq_restore(self.irq_state);
        }
    }
}

unsafe impl<T: ?Sized> Sync for IrqSafeNullLock<T> {}
