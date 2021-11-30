use core::convert::TryFrom;
use crate::error::Errno;

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
#[non_exhaustive]
pub enum IoctlCmd {
    TtySetAttributes = 1,
    TtyGetAttributes = 2,
    TtySetPgrp = 3,
}

impl TryFrom<u32> for IoctlCmd {
    type Error = Errno;

    #[inline]
    fn try_from(u: u32) -> Result<IoctlCmd, Errno> {
        match u {
            1 => Ok(Self::TtySetAttributes),
            2 => Ok(Self::TtyGetAttributes),
            3 => Ok(Self::TtySetPgrp),
            _ => Err(Errno::InvalidArgument)
        }
    }
}
