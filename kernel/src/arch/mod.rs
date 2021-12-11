//! Architecture-specific detail module
//!
//! Contains two module aliases, which may or may not point
//! the same architecture module:
//!
//! * [platform] - architecture details (e.g. aarch64)
//! * [machine] - particular machine implementation (e.g. bcm2837)
//!
//! Modules visible in the documentation will depend on
//! build target platform.

cfg_if! {
    if #[cfg(target_arch = "aarch64")] {
        pub mod aarch64;

        pub use aarch64 as platform;
        pub use aarch64::{machine, intrin};
    } else if #[cfg(target_arch = "x86_64")] {
        pub mod x86_64;

        pub use x86_64 as platform;
        pub use x86_64 as machine;
        pub use x86_64::intrin;
    }
}

// TODO move to mod io
// use core::marker::PhantomData;
// use core::ops::Deref;
//
// /// Wrapper for setting up memory-mapped registers and IO
// pub struct MemoryIo<T> {
//     base: usize,
//     _pd: PhantomData<fn() -> T>,
// }
//
// impl<T> MemoryIo<T> {
//     /// Constructs a new instance of MMIO region.
//     ///
//     /// # Safety
//     ///
//     /// Does not perform `base` validation.
//     pub const unsafe fn new(base: usize) -> Self {
//         Self {
//             base,
//             _pd: PhantomData,
//         }
//     }
// }
//
// impl<T> Deref for MemoryIo<T> {
//     type Target = T;
//
//     #[inline(always)]
//     fn deref(&self) -> &Self::Target {
//         unsafe { &*(self.base as *const _) }
//     }
// }
