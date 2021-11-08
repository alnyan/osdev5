bitflags! {
    pub struct TermiosIflag: u32 {
        /// Translate NL to CR on input
        const INLCR = 1 << 0;
        /// Translate CR to NL on input
        const ICRNL = 1 << 1;
    }

    pub struct TermiosOflag: u32 {
        /// Translate NL to CR-NL on output
        const ONLCR = 1 << 0;
    }

    pub struct TermiosLflag: u32 {
        /// Signal processing (INTR, QUIT, SUSP)
        const ISIG = 1 << 0;
        /// Canonical mode
        const ICANON = 1 << 1;
        /// Echo input characters
        const ECHO = 1 << 2;
        /// If ICANON also set, ERASE erases chars, WERASE erases words
        const ECHOE = 1 << 3;
        /// If ICANON also set, KILL erases line
        const ECHOK = 1 << 4;
        /// If ICANON also set, echo NL even if ECHO is not set
        const ECHONL = 1 << 5;
    }
}

#[derive(Debug, Clone)]
pub struct TermiosChars {
    pub eof: u8,
    pub erase: u8,
    pub intr: u8,
    pub kill: u8,
    pub vlnext: u8,
    pub werase: u8,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Termios {
    pub iflag: TermiosIflag,
    pub oflag: TermiosOflag,
    pub lflag: TermiosLflag,
    pub chars: TermiosChars
}

impl TermiosChars {
    pub const fn new() -> Self {
        Self {
            eof: 0x04,
            erase: 0x7F,
            intr: 0x03,
            kill: 0x15,
            vlnext: 0x16,
            werase: 0x17,
        }
    }
}

impl Termios {
    pub const fn new() -> Self {
        Self {
            iflag: TermiosIflag::ICRNL,
            oflag: TermiosOflag::ONLCR,
            // TODO prettify this
            lflag: unsafe {
                TermiosLflag::from_bits_unchecked(
                    (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4) | (1 << 5),
                )
            },
            chars: TermiosChars::new()
        }
    }

    pub const fn is_canon(&self) -> bool {
        self.lflag.contains(TermiosLflag::ICANON)
    }
}
