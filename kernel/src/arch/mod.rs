cfg_if! {
    if #[cfg(target_arch = "aarch64")] {
        pub mod aarch64;

        pub use aarch64 as platform;
        pub use aarch64::machine;
    }
}

// TODO move to mod io
use core::ops::Deref;
use core::marker::PhantomData;

pub struct MemoryIo<T> {
    base: usize,
    _pd: PhantomData<fn() -> T>,
}

impl<T> MemoryIo<T> {
    pub const unsafe fn new(base: usize) -> Self {
        Self {
            base,
            _pd: PhantomData
        }
    }
}

impl<T> Deref for MemoryIo<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self.base as *const _) }
    }
}
