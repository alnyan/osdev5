//! Module for device interfaces and drivers

use error::Errno;

// Device classes
pub mod serial;
pub mod timer;
pub mod gpio;
pub mod irq;

/// Generic device trait
pub trait Device {
    /// Returns device type/driver name
    fn name(&self) -> &'static str;

    /// Performs device initialization logic.
    ///
    /// # Safety
    ///
    /// Marked unsafe as it may cause direct hardware-specific side-effects.
    /// Additionally, may be called twice with undefined results.
    unsafe fn enable(&self) -> Result<(), Errno>;
}
