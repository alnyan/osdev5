use core::fmt;

#[derive(Clone, Copy, PartialEq)]
#[repr(transparent)]
pub struct FileMode(u16);

impl FileMode {
    const USER_READ: u16 = 1 << 8;
    const USER_WRITE: u16 = 1 << 7;
    const USER_EXEC: u16 = 1 << 6;
    const GROUP_READ: u16 = 1 << 5;
    const GROUP_WRITE: u16 = 1 << 4;
    const GROUP_EXEC: u16 = 1 << 3;
    const OTHER_READ: u16 = 1 << 2;
    const OTHER_WRITE: u16 = 1 << 1;
    const OTHER_EXEC: u16 = 1 << 0;

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn default_dir() -> Self {
        Self(0o755)
    }

    pub const fn default_reg() -> Self {
        Self(0o644)
    }
}

impl fmt::Debug for FileMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FileMode (")?;

        if self.0 & Self::USER_READ != 0 {
            write!(f, "r")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::USER_WRITE != 0 {
            write!(f, "w")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::USER_EXEC != 0 {
            write!(f, "x")?;
        } else {
            write!(f, "-")?;
        }

        if self.0 & Self::GROUP_READ != 0 {
            write!(f, "r")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::GROUP_WRITE != 0 {
            write!(f, "w")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::GROUP_EXEC != 0 {
            write!(f, "x")?;
        } else {
            write!(f, "-")?;
        }

        if self.0 & Self::OTHER_READ != 0 {
            write!(f, "r")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::OTHER_WRITE != 0 {
            write!(f, "w")?;
        } else {
            write!(f, "-")?;
        }
        if self.0 & Self::OTHER_EXEC != 0 {
            write!(f, "x")?;
        } else {
            write!(f, "-")?;
        }

        write!(f, ")")?;
        Ok(())
    }
}
