use crate::error::Errno;
use crate::proc::{Pid, Pgid};

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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SignalDestination {
    Group(Pgid),
    Process(Pid),
    All,
    This
}

impl From<isize> for SignalDestination {
    fn from(num: isize) -> Self {
        if num > 0 {
            Self::Process(Pid::user(num as u32))
        } else if num == 0 {
            Self::This
        } else if num == -1 {
            Self::All
        } else {
            Self::Group(Pgid::from((-num) as u32))
        }
    }
}

impl From<SignalDestination> for isize {
    fn from(p: SignalDestination) -> isize {
        match p {
            SignalDestination::Process(pid) => u32::from(pid) as isize,
            SignalDestination::Group(pgid) => -(u32::from(pgid) as isize),
            SignalDestination::This => 0,
            SignalDestination::All => -1
        }
    }
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
