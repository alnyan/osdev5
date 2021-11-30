//! Teletype (TTY) device facilities
use crate::dev::serial::SerialDevice;
use crate::proc::{Process, wait::{Wait, WAIT_SELECT}};
use crate::sync::IrqSafeSpinLock;
use libsys::error::Errno;
use libsys::{
    termios::{Termios, TermiosIflag, TermiosLflag, TermiosOflag},
    proc::Pid,
    signal::Signal,
    ioctl::IoctlCmd
};
use core::mem::size_of;
use crate::syscall::arg;

#[derive(Debug)]
struct CharRingInner<const N: usize> {
    rd: usize,
    wr: usize,
    data: [u8; N],
    flags: u8,
    fg_pgid: Option<Pid>,
}

/// Ring buffer for TTYs
pub struct CharRing<const N: usize> {
    wait_read: Wait,
    wait_write: Wait,
    config: IrqSafeSpinLock<Termios>,
    inner: IrqSafeSpinLock<CharRingInner<N>>,
}

/// Generic teletype device interface
pub trait TtyDevice<const N: usize>: SerialDevice {
    /// Returns a reference to character device's ring buffer
    fn ring(&self) -> &CharRing<N>;

    /// Returns `true` if the TTY is ready for an operation
    fn is_ready(&self, write: bool) -> Result<bool, Errno> {
        let ring = self.ring();
        if write {
            todo!()
        } else {
            Ok(ring.is_readable())
        }
    }

    /// Performs a TTY control request
    fn tty_ioctl(&self, cmd: IoctlCmd, ptr: usize, _len: usize) -> Result<usize, Errno> {
        match cmd {
            IoctlCmd::TtyGetAttributes => {
                // TODO validate size
                let res = arg::struct_mut::<Termios>(ptr)?;
                *res = self.ring().config.lock().clone();
                Ok(size_of::<Termios>())
            },
            IoctlCmd::TtySetAttributes => {
                let src = arg::struct_ref::<Termios>(ptr)?;
                *self.ring().config.lock() = src.clone();
                Ok(size_of::<Termios>())
            },
            IoctlCmd::TtySetPgrp => {
                let src = arg::struct_ref::<u32>(ptr)?;
                self.ring().inner.lock().fg_pgid = Some(Pid::try_from(*src)?);
                Ok(0)
            },
            _ => Err(Errno::InvalidArgument)
        }
    }

    /// Processes and writes output an output byte
    fn line_send(&self, byte: u8) -> Result<(), Errno> {
        let config = self.ring().config.lock();

        if byte == b'\n' && config.oflag.contains(TermiosOflag::ONLCR) {
            self.send(b'\r').ok();
        }

        self.send(byte)
    }

    /// Receives input bytes and processes them
    fn recv_byte(&self, mut byte: u8) {
        let ring = self.ring();
        let config = ring.config.lock();

        if byte == b'@' {
            use crate::mem::phys;
            let stat = phys::statistics();
            debugln!("Physical memory stats:");
            debugln!("{:#?}", stat);
            return;
        }

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

        if byte == 0x3 && config.lflag.contains(TermiosLflag::ISIG) {
            drop(config);
            let pgid = ring.inner.lock().fg_pgid;
            if let Some(pgid) = pgid {
                // TODO send to pgid
                let proc = Process::get(pgid);
                if let Some(proc) = proc {
                    proc.set_signal(Signal::Interrupt);
                }
            }
            return;
        }

        self.ring().putc(byte, false).ok();
    }

    /// Line discipline function
    fn line_read(&self, data: &mut [u8]) -> Result<usize, Errno> {
        let ring = self.ring();
        let mut config = ring.config.lock();

        if data.is_empty() {
            return Ok(0);
        }

        if !config.is_canon() {
            drop(config);
            let byte = ring.getc()?;
            data[0] = byte;
            Ok(1)
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

                        for _ in idx..len {
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

    /// Processes and writes string bytes
    fn line_write(&self, data: &[u8]) -> Result<usize, Errno> {
        for &byte in data.iter() {
            self.line_send(byte)?;
        }
        Ok(data.len())
    }

    /// Writes string bytes without any processing
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
                fg_pgid: None,
                rd: 0,
                wr: 0,
                data: [0; N],
                flags: 0,
            }),
            config: IrqSafeSpinLock::new(Termios::new()),
            wait_read: Wait::new("tty_read"),
            wait_write: Wait::new("tty_write"),
        }
    }

    /// Returns `true` if a character/line is available for reception
    pub fn is_readable(&self) -> bool {
        let inner = self.inner.lock();
        let config = self.config.lock();
        if config.lflag.contains(TermiosLflag::ICANON) {
            // TODO optimize this somehow?
            let mut rd = inner.rd;
            let mut count = 0usize;
            loop {
                let readable = if rd <= inner.wr {
                    (inner.wr - rd) > 0
                } else {
                    (inner.wr + (N - rd)) > 0
                };

                if !readable {
                    break;
                }

                let byte = inner.data[rd];
                if byte == b'\n' || byte == config.chars.eof {
                    count += 1;
                }

                rd = (rd + 1) % N;
            }

            count != 0 || inner.flags != 0
        } else {
            inner.is_readable() || inner.flags != 0
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
        let byte = lock.read_unchecked();
        drop(lock);
        self.wait_write.wakeup_one();
        WAIT_SELECT.wakeup_all();
        Ok(byte)
    }

    /// Puts a single byte to the buffer
    pub fn putc(&self, ch: u8, blocking: bool) -> Result<(), Errno> {
        let mut lock = self.inner.lock();
        if blocking {
            todo!()
        }
        lock.write_unchecked(ch);
        drop(lock);
        self.wait_read.wakeup_one();
        WAIT_SELECT.wakeup_all();
        Ok(())
    }
}
