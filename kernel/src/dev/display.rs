use crate::dev::Device;
use libsys::error::Errno;
use crate::util::InitOnce;

pub struct FramebufferInfo {
    pub width: usize,
    pub height: usize,
    pub phys_base: usize,
    pub virt_base: usize
}

pub trait Display: Device {
    fn set_mode(&self, mode: DisplayMode) -> Result<(), Errno>;
    fn framebuffer<'a>(&'a self) -> Result<&'a FramebufferInfo, Errno>;
}

pub struct DisplayMode {
    width: u16,
    height: u16,
}

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
    fn set_mode(&self, mode: DisplayMode) -> Result<(), Errno> {
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
    pub const fn uninit() -> Self {
        Self { framebuffer: InitOnce::new() }
    }

    pub fn set_framebuffer(&self, framebuffer: FramebufferInfo) {
        self.framebuffer.init(framebuffer);
    }
}
