//! Module for device interfaces and drivers

use error::Errno;

// Device classes
pub mod serial;
pub mod timer;
pub mod gpio;

/// Generic device trait
pub trait Device {
    /// Returns device type/driver name
    fn name() -> &'static str;

    /// Performs device initialization logic.
    ///
    /// # Safety
    ///
    /// Marked unsafe as it may cause direct hardware-specific side-effects.
    /// Additionally, may be called twice with undefined results.
    unsafe fn enable(&mut self) -> Result<(), Errno>;
}
