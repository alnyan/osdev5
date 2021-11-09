//! Teletype (TTY) device facilities
use crate::dev::serial::SerialDevice;
use crate::proc::wait::Wait;
use crate::sync::IrqSafeSpinLock;
use error::Errno;
use syscall::{
    termios::{Termios, TermiosIflag, TermiosLflag, TermiosOflag},
    ioctl::IoctlCmd
};
use core::mem::size_of;
use crate::syscall::arg::validate_user_ptr_struct;
use vfs::CharDevice;

#[derive(Debug)]
struct CharRingInner<const N: usize> {
    rd: usize,
    wr: usize,
    data: [u8; N],
    flags: u8,
}

/// Ring buffer for TTYs
pub struct CharRing<const N: usize> {
    wait_read: Wait,
    wait_write: Wait,
    config: IrqSafeSpinLock<Termios>,
    inner: IrqSafeSpinLock<CharRingInner<N>>,
}

pub trait TtyDevice<const N: usize>: SerialDevice {
    fn ring(&self) -> &CharRing<N>;

    fn tty_ioctl(&self, cmd: IoctlCmd, ptr: usize, len: usize) -> Result<usize, Errno> {
        match cmd {
            IoctlCmd::TtyGetAttributes => {
                // TODO validate size
                let res = validate_user_ptr_struct::<Termios>(ptr)?;
                *res = self.ring().config.lock().clone();
                Ok(size_of::<Termios>())
            },
            IoctlCmd::TtySetAttributes => {
                let src = validate_user_ptr_struct::<Termios>(ptr)?;
                *self.ring().config.lock() = src.clone();
                Ok(size_of::<Termios>())
            },
            _ => Err(Errno::InvalidArgument)
        }
    }

    fn line_send(&self, byte: u8) -> Result<(), Errno> {
        let config = self.ring().config.lock();

        if byte == b'\n' && config.oflag.contains(TermiosOflag::ONLCR) {
            self.send(b'\r').ok();
        }

        self.send(byte)
    }

    fn recv_byte(&self, mut byte: u8) {
        let ring = self.ring();
        let config = ring.config.lock();

        if byte == b'\r' && config.iflag.contains(TermiosIflag::ICRNL) {
            byte = b'\n';
        }

        if byte == b'\n' {
            if config.lflag.contains(TermiosLflag::ECHO)
                || (config.is_canon() && config.lflag.contains(TermiosLflag::ECHONL))
            {
                if byte == b'\n' && config.oflag.contains(TermiosOflag::ONLCR) {
                    self.send(b'\r').ok();
                }
                self.send(byte).ok();
            }
        } else if config.lflag.contains(TermiosLflag::ECHO) {
            let echoe = (byte == config.chars.erase || byte == config.chars.werase)
                && config.lflag.contains(TermiosLflag::ECHOE);
            let echok = byte == config.chars.kill && config.lflag.contains(TermiosLflag::ECHOE);

            if byte.is_ascii_control() {
                if !echoe && !echok {
                    self.send(b'^').ok();
                    self.send(byte + 0x40).ok();
                }
            } else {
                self.send(byte).ok();
            }
        }

        self.ring().putc(byte, false).ok();
    }

    /// Line discipline function
    fn line_read(&self, data: &mut [u8]) -> Result<usize, Errno> {
        let ring = self.ring();
        let mut config = ring.config.lock();

        if data.len() == 0 {
            return Ok(0);
        }

        if !config.is_canon() {
            drop(config);
            let byte = ring.getc()?;
            data[0] = byte;
            return Ok(1);
        } else {
            let mut rem = data.len();
            let mut off = 0;
            // Perform canonical read
            while rem != 0 {
                drop(config);
                let byte = ring.getc()?;
                config = ring.config.lock();

                if byte == config.chars.eof && config.is_canon() {
                    break;
                }
                if byte == config.chars.erase && config.is_canon() {
                    if off > 0 && config.lflag.contains(TermiosLflag::ECHOE) {
                        self.raw_write(b"\x1B[D \x1B[D").ok();
                        off -= 1;
                        rem += 1;
                    }

                    continue;
                }
                if byte == config.chars.werase && config.is_canon() {
                    if off > 0 && config.lflag.contains(TermiosLflag::ECHOE) {
                        let idx = data[..off].iter().rposition(|&ch| ch == b' ').unwrap_or(0);
                        let len = off;

                        for i in idx..len {
                            self.raw_write(b"\x1B[D \x1B[D").ok();
                            off -= 1;
                            rem += 1;
                        }
                    }

                    continue;
                }
                if byte == config.chars.kill && config.is_canon() {
                    if off > 0 && config.lflag.contains(TermiosLflag::ECHOK) {
                        while off != 0 {
                            self.raw_write(b"\x1B[D \x1B[D").ok();
                            off -= 1;
                            rem += 1;
                        }
                    }

                    continue;
                }

                data[off] = byte;
                off += 1;
                rem -= 1;

                if byte == b'\n' || byte == b'\r' {
                    break;
                }
            }
            Ok(off)
        }
    }

    fn line_write(&self, data: &[u8]) -> Result<usize, Errno> {
        for &byte in data.iter() {
            self.line_send(byte)?;
        }
        Ok(data.len())
    }

    fn raw_write(&self, data: &[u8]) -> Result<usize, Errno> {
        for &byte in data.iter() {
            self.send(byte)?;
        }
        Ok(data.len())
    }
}

impl<const N: usize> CharRingInner<N> {
    #[inline]
    const fn is_readable(&self) -> bool {
        if self.rd <= self.wr {
            (self.wr - self.rd) > 0
        } else {
            (self.wr + (N - self.rd)) > 0
        }
    }

    #[inline]
    fn read_unchecked(&mut self) -> u8 {
        let res = self.data[self.rd];
        self.rd = (self.rd + 1) % N;
        res
    }

    #[inline]
    fn write_unchecked(&mut self, ch: u8) {
        self.data[self.wr] = ch;
        self.wr = (self.wr + 1) % N;
    }
}

impl<const N: usize> CharRing<N> {
    /// Returns a new fixed-size ring buffer
    pub const fn new() -> Self {
        Self {
            inner: IrqSafeSpinLock::new(CharRingInner {
                rd: 0,
                wr: 0,
                data: [0; N],
                flags: 0,
            }),
            config: IrqSafeSpinLock::new(Termios::new()),
            wait_read: Wait::new(),
            wait_write: Wait::new(),
        }
    }

    /// Performs a blocking read of a single byte from the buffer
    pub fn getc(&self) -> Result<u8, Errno> {
        let mut lock = self.inner.lock();
        loop {
            if !lock.is_readable() && lock.flags == 0 {
                drop(lock);
                self.wait_read.wait(None)?;
                lock = self.inner.lock();
            } else {
                break;
            }
        }
        if lock.flags != 0 {
            if lock.flags & (1 << 0) != 0 {
                lock.flags &= !(1 << 0);
                return Err(Errno::EndOfFile);
            }
            todo!();
        }
        let byte = lock.read_unchecked();
        self.wait_write.wakeup_one();
        Ok(byte)
    }

    /// Puts a single byte to the buffer
    pub fn putc(&self, ch: u8, blocking: bool) -> Result<(), Errno> {
        let mut lock = self.inner.lock();
        if blocking {
            todo!()
        }
        lock.write_unchecked(ch);
        self.wait_read.wakeup_one();
        Ok(())
    }
}
