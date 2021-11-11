use core::convert::TryFrom;
use crate::error::Errno;

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
pub enum IoctlCmd {
    TtySetAttributes = 1,
    TtyGetAttributes = 2,
}

impl TryFrom<u32> for IoctlCmd {
    type Error = Errno;

    #[inline]
    fn try_from(u: u32) -> Result<IoctlCmd, Errno> {
        match u {
            1 => Ok(Self::TtySetAttributes),
            2 => Ok(Self::TtyGetAttributes),
            _ => Err(Errno::InvalidArgument)
        }
    }
}
