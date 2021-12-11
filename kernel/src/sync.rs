//! Synchronization facilities module

use crate::arch::platform::{irq_mask_save, irq_restore};
use core::cell::UnsafeCell;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

/// Lock structure ensuring IRQs are disabled when inner value is accessed
pub struct IrqSafeSpinLock<T> {
    value: UnsafeCell<T>,
    state: AtomicBool,
}

/// Guard-structure wrapping a reference to value owned by [IrqSafeSpinLock].
/// Restores saved IRQ state when dropped.
pub struct IrqSafeSpinLockGuard<'a, T> {
    lock: &'a IrqSafeSpinLock<T>,
    irq_state: u64,
}

impl<T> IrqSafeSpinLock<T> {
    /// Constructs a new instance of the lock, wrapping `value`
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            state: AtomicBool::new(false),
        }
    }

    #[inline(always)]
    fn try_lock(&self) -> Result<bool, bool> {
        self.state
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
    }

    #[inline(always)]
    unsafe fn force_release(&self) {
        self.state.store(false, Ordering::Release);
        // cortex_a::asm::sev();
    }

    /// Returns [IrqSafeSpinLockGuard] for this lock
    #[inline]
    pub fn lock(&self) -> IrqSafeSpinLockGuard<T> {
        let irq_state = unsafe { irq_mask_save() };

        while self.try_lock().is_err() {
            // cortex_a::asm::wfe();
        }

        IrqSafeSpinLockGuard {
            lock: self,
            irq_state,
        }
    }
}

impl<T> Deref for IrqSafeSpinLockGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for IrqSafeSpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T: fmt::Debug> fmt::Debug for IrqSafeSpinLockGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(unsafe { &*self.lock.value.get() }, f)
    }
}

impl<T> Drop for IrqSafeSpinLockGuard<'_, T> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            self.lock.force_release();
            irq_restore(self.irq_state);
        }
    }
}

unsafe impl<T> Sync for IrqSafeSpinLock<T> {}
