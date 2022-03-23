//! Graphical display interfaces
use crate::dev::Device;
use libsys::error::Errno;
use crate::util::InitOnce;

/// Description of a framebuffer
pub struct FramebufferInfo {
    /// Width in pixels
    pub width: usize,
    /// Height in pixels
    pub height: usize,
    /// Physical start address
    pub phys_base: usize,
    /// Virtual address where the framebuffer is mapped
    pub virt_base: usize
}

/// Generic display interface
pub trait Display: Device {
    /// Changes currently active display mode
    fn set_mode(&self, mode: DisplayMode) -> Result<(), Errno>;
    /// Returns currently active framebuffer information
    fn framebuffer(&self) -> Result<&FramebufferInfo, Errno>;
}

/// Display configuration details
#[allow(dead_code)]
pub struct DisplayMode {
    width: u16,
    height: u16,
}

/// Generic single-mode framebuffer
pub struct StaticFramebuffer {
    framebuffer: InitOnce<FramebufferInfo>
}

impl Device for StaticFramebuffer {
    fn name(&self) -> &'static str {
        "Generic framebuffer device"
    }

    unsafe fn enable(&self) -> Result<(), Errno> {
        Ok(())
    }
}

impl Display for StaticFramebuffer {
    fn set_mode(&self, _mode: DisplayMode) -> Result<(), Errno> {
        Err(Errno::InvalidOperation)
    }

    fn framebuffer(&self) -> Result<&FramebufferInfo, Errno> {
        if let Some(fb) = self.framebuffer.as_ref_option() {
            Ok(fb)
        } else {
            Err(Errno::InvalidOperation)
        }
    }
}

impl StaticFramebuffer {
    /// Constructs an empty [StaticFramebuffer] object
    pub const fn uninit() -> Self {
        Self { framebuffer: InitOnce::new() }
    }

    /// Initializes the device from existing framebuffer object
    pub fn set_framebuffer(&self, framebuffer: FramebufferInfo) {
        self.framebuffer.init(framebuffer);
    }
}
