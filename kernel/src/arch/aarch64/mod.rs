//! aarch64 architecture implementation

pub mod boot;
pub mod timer;
pub mod asm;
pub mod exception;

cfg_if! {
    if #[cfg(feature = "mach_qemu")] {
        pub mod mach_qemu;

        pub use mach_qemu as machine;
    } else if #[cfg(feature = "mach_orangepi3")] {
        pub mod mach_orangepi3;

        pub use mach_orangepi3 as machine;
    }
}
