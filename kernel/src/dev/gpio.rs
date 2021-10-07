//! GPIO and pin control interfaces

use crate::dev::Device;
use error::Errno;

/// Pin function mode
pub enum PinMode {
    /// Do not use pin
    Disable = 0,
    /// Use pin as a GPIO input
    Input,
    /// Use pin as a GPIO output
    Output,
    /// Use pin as an external interrupt trigger source
    InputInterrupt,
    /// Use pin for peripheral functionality
    Alt,
}

/// Input/output pin pull mode
pub enum PullMode {
    /// No pull
    None,
    /// Pull up
    Up,
    /// Pull down
    Down,
}

/// Pin configuration for [GpioDevice::set_pin_config]
pub struct PinConfig {
    /// Pin function
    pub mode: PinMode,
    /// Pin pull mode, only used for Input/Output pins
    pub pull: PullMode,
    /// Alternate pin function, only used when mode == [PinMode::Alt]
    pub func: u32,
}

// TODO separate traits for "single port controller" and "global gpio controller"
/// Generic GPIO controller interface
pub trait GpioDevice: Device {
    /// Initializes configuration for given pin
    ///
    /// # Safety
    ///
    /// Unsafe: changes physical pin configuration
    unsafe fn set_pin_config(&self, pin: u32, cfg: &PinConfig) -> Result<(), Errno>;
    /// Returns current configuration of given pin
    fn get_pin_config(&self, pin: u32) -> Result<PinConfig, Errno>;

    /// Sets `pin` to HIGH state
    fn set_pin(&self, pin: u32);
    /// Sets `pin` to LOW state
    fn clear_pin(&self, pin: u32);
    /// Toggles `pin`'s HIGH/LOW state
    fn toggle_pin(&self, pin: u32);
    /// Returns `true` if input `pin` is in HIGH state
    fn read_pin(&self, pin: u32) -> Result<bool, Errno>;
}

impl PinConfig {
    /// Alternative (peripheral) pin configuration
    pub const fn alt(func: u32) -> Self {
        Self {
            mode: PinMode::Alt,
            pull: PullMode::None,
            func
        }
    }

    /// Pull-down output
    pub const fn out_pull_down() -> Self {
        Self {
            mode: PinMode::Output,
            pull: PullMode::Down,
            func: 0
        }
    }
}
