//! Teletype (TTY) device facilities
use crate::proc::wait::Wait;
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

    /// Line discipline function
    pub fn line_read<T: CharDevice>(&self, data: &mut [u8], dev: &T) -> Result<usize, Errno> {
        let mut rem = data.len();
        let mut off = 0;

        while rem != 0 {
            let byte = match self.getc() {
                Ok(ch) => ch,
                Err(Errno::Interrupt) => {
                    todo!()
                }
                Err(e) => return Err(e),
            };

            if byte == b'\n' || byte == b'\r' {
                dev.write(true, b"\r\n").ok();
                break;
            }

            if byte == 0x7F {
                if off > 0 {
                    dev.write(true, b"\x1B[D \x1B[D").ok();
                    off -= 1;
                    rem += 1;
                }
                continue;
            } else if byte >= b' ' {
                // TODO tty options
                dev.write(true, &[byte]).ok();
            }

            data[off] = byte;
            off += 1;
            rem -= 1;
        }
        Ok(off)
    }
}
