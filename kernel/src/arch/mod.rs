cfg_if! {
    if #[cfg(target_arch = "aarch64")] {
        pub mod aarch64;

        pub use aarch64 as platform;
    }
}
