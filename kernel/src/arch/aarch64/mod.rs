//! aarch64 architecture implementation

pub mod boot;

cfg_if! {
    if #[cfg(feature = "mach_qemu")] {
        pub mod mach_qemu;

        pub use mach_qemu as machine;
    }
}
