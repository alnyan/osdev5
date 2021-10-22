//! Synchronization facilities module

use crate::arch::platform::{irq_mask_save, irq_restore};
use core::cell::UnsafeCell;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};
use cortex_a::registers::MPIDR_EL1;
use tock_registers::interfaces::Readable;

/// Lock structure ensuring IRQs are disabled when inner value is accessed
pub struct IrqSafeSpinLock<T> {
    value: UnsafeCell<T>,
    state: AtomicUsize,
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
            state: AtomicUsize::new(usize::MAX),
        }
    }

    /// Returns [IrqSafeSpinLockGuard] for this lock
    #[inline]
    pub fn lock(&self) -> IrqSafeSpinLockGuard<T> {
        let irq_state = unsafe { irq_mask_save() };
        let id = MPIDR_EL1.get() & 0xF;

        while let Err(e) = self.state.compare_exchange_weak(
            usize::MAX,
            id as usize,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            // if e == id as usize {
            //     break;
            // }
            cortex_a::asm::wfe();
        }

        IrqSafeSpinLockGuard {
            lock: self,
            irq_state,
        }
    }

    pub unsafe fn force_release(&self) {
        self.state.store(usize::MAX, Ordering::Release);
        cortex_a::asm::sev();
    }
}

impl<T> Deref for IrqSafeSpinLockGuard<'_, T> {
    type Target = T;

    #[inline(always)]
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
