//! aarch64 architecture implementation

pub mod boot;
pub mod timer;
pub mod asm;

cfg_if! {
    if #[cfg(feature = "mach_qemu")] {
        pub mod mach_qemu;

        pub use mach_qemu as machine;
    }
}
