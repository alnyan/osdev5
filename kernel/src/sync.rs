use core::ops::{Deref, DerefMut};
use core::cell::UnsafeCell;

#[repr(transparent)]
pub struct NullLock<T: ?Sized> {
    value: UnsafeCell<T>
}

#[repr(transparent)]
pub struct NullLockGuard<'a, T: ?Sized> {
    value: &'a mut T
}

impl<T> NullLock<T> {
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self { value: UnsafeCell::new(value) }
    }

    #[inline(always)]
    pub fn lock(&self) -> NullLockGuard<T> {
        NullLockGuard { value: unsafe { &mut *self.value.get() } }
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
