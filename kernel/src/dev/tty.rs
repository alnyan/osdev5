//! Teletype (TTY) device facilities
use crate::proc::wait::Wait;
use crate::dev::serial::SerialDevice;
use crate::sync::IrqSafeSpinLock;
use error::Errno;
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
    inner: IrqSafeSpinLock<CharRingInner<N>>,
}

pub trait TtyDevice<const N: usize>: SerialDevice {
    fn ring(&self) -> &CharRing<N>;

    fn line_send(&self, byte: u8) -> Result<(), Errno> {
        if byte == b'\n' {
            self.send(b'\r').ok();
        }

        self.send(byte)
    }

    fn recv_byte(&self, mut byte: u8) {
        if byte == b'\r' {
            // TODO ICRNL conv option
            byte = b'\n';
        }

        if byte == 4 {
            // TODO handle EOF
            self.ring().signal_eof();
            return;
        }

        self.ring().putc(byte, false).ok();

        match byte {
            b'\n' => {
                // TODO ECHONL option
                self.line_send(byte).ok();
                // TODO ICANON
            }
            0x17 | 0x7F => (),
            0x0C => {
                // TODO ECHO option && ICANON option
                self.raw_write(b"^L").ok();
            },
            0x1B => {
                // TODO ECHO option && ICANON option
                self.raw_write(b"^[").ok();
            },
            _ => {
                // TODO ECHO option
                self.line_send(byte).ok();
            }
        }
    }

    /// Line discipline function
    fn line_read(&self, data: &mut [u8]) -> Result<usize, Errno> {
        let ring = self.ring();
        let mut rem = data.len();
        let mut off = 0;

        while rem != 0 {
            let byte = match ring.getc() {
                Ok(ch) => ch,
                Err(Errno::Interrupt) => {
                    todo!()
                }
                Err(Errno::EndOfFile) => {
                    break;
                }
                Err(e) => return Err(e),
            };

            if byte == 0x7F {
                if off > 0 {
                    self.raw_write(b"\x1B[D \x1B[D").ok();
                    off -= 1;
                    rem += 1;
                }
                continue;
            } else if byte >= b' ' {
                // TODO tty options
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

    fn signal_eof(&self) {
        self.inner.lock().flags |= 1 << 0;
        self.wait_read.wakeup_one();
    }
}
