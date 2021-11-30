//! GPIO and pin control interfaces

use crate::dev::Device;
use libsys::error::Errno;

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

/// Generic GPIO controller interface
pub trait GpioDevice: Device {
    /// Controller-specific address type for a single pin,
    /// may include its bank and pin numbers
    type PinAddress;

    /// Initializes configuration for given pin
    ///
    /// # Safety
    ///
    /// Unsafe: changes physical pin configuration
    unsafe fn set_pin_config(&self, pin: Self::PinAddress, cfg: &PinConfig) -> Result<(), Errno>;
    /// Returns current configuration of given pin
    fn get_pin_config(&self, pin: Self::PinAddress) -> Result<PinConfig, Errno>;

    /// Sets `pin` to HIGH/LOW `state`
    fn write_pin(&self, pin: Self::PinAddress, state: bool);
    /// Toggles `pin`'s HIGH/LOW state
    fn toggle_pin(&self, pin: Self::PinAddress);
    /// Returns `true` if input `pin` is in HIGH state
    fn read_pin(&self, pin: Self::PinAddress) -> Result<bool, Errno>;
}

impl PinConfig {
    /// Alternative (peripheral) pin configuration
    pub const fn alt(func: u32) -> Self {
        Self {
            mode: PinMode::Alt,
            pull: PullMode::None,
            func,
        }
    }

    /// Pull-down output
    pub const fn out_pull_down() -> Self {
        Self {
            mode: PinMode::Output,
            pull: PullMode::Down,
            func: 0,
        }
    }
}
