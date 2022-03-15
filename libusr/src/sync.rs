use crate::sys::RawMutex;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

pub struct Mutex<T> {
    inner: RawMutex,
    data: UnsafeCell<T>,
}

pub struct MutexGuard<'a, T> {
    data: &'a mut T,
    lock: &'a RawMutex,
}

impl<T> Mutex<T> {
    pub fn new(t: T) -> Self {
        Self {
            inner: RawMutex::new(),
            data: UnsafeCell::new(t),
        }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        unsafe {
            self.inner.lock();
            MutexGuard {
                data: (&mut *self.data.get()),
                lock: &self.inner,
            }
        }
    }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.lock.release();
        }
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

unsafe impl<T> Sync for Mutex<T> {}
