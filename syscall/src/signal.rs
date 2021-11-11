use error::Errno;

#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u32)]
pub enum Signal {
    Interrupt = 2,
    IllegalInstruction = 4,
    FloatError = 8,
    Kill = 9,
    SegmentationFault = 11,
    InvalidSystemCall = 31
}

impl TryFrom<u32> for Signal {
    type Error = Errno;

    #[inline]
    fn try_from(u: u32) -> Result<Self, Errno> {
        match u {
            2 => Ok(Self::Interrupt),
            4 => Ok(Self::IllegalInstruction),
            8 => Ok(Self::FloatError),
            9 => Ok(Self::Kill),
            11 => Ok(Self::SegmentationFault),
            31 => Ok(Self::InvalidSystemCall),
            _ => Err(Errno::InvalidArgument)
        }
    }
}
