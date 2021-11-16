use libsys::{
    stat::FileDescriptor,
    calls::{sys_read, sys_write}
};
use crate::io::{Read, Write, Error};
use crate::sync::{Mutex, MutexGuard};
use core::fmt;

struct InputInner {
    fd: FileDescriptor
}
struct OutputInner {
    fd: FileDescriptor
}

pub struct StdinLock<'a> {
    lock: MutexGuard<'a, InputInner>
}

pub struct StdoutLock<'a> {
    lock: MutexGuard<'a, OutputInner>
}

pub struct StderrLock<'a> {
    lock: MutexGuard<'a, OutputInner>
}

pub struct Stdin {
    inner: &'static Mutex<InputInner>,
}

pub struct Stdout {
    inner: &'static Mutex<OutputInner>
}

pub struct Stderr {
    inner: &'static Mutex<OutputInner>
}

// STDIN

impl Read for InputInner {
    fn read(&mut self, bytes: &mut [u8]) -> Result<usize, Error> {
        sys_read(self.fd, bytes).map_err(Error::from)
    }
}

impl Read for Stdin {
    fn read(&mut self, bytes: &mut [u8]) -> Result<usize, Error> {
        self.inner.lock().read(bytes)
    }
}

// STDOUT/STDERR

impl fmt::Write for OutputInner {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write(s.as_bytes()).map(|_| ()).map_err(|_| todo!())
    }
}

impl Write for OutputInner {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        sys_write(self.fd, bytes).map_err(Error::from)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<(), Error> {
        fmt::Write::write_fmt(self, args).map_err(|_| todo!())
    }
}

impl Write for Stdout {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        self.inner.lock().write(bytes)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<(), Error> {
        self.inner.lock().write_fmt(args)
    }
}

impl Write for Stderr {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        self.inner.lock().write(bytes)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<(), Error> {
        self.inner.lock().write_fmt(args)
    }
}

impl Stdout {
    pub fn lock(&self) -> StdoutLock {
        StdoutLock {
            lock: self.inner.lock()
        }
    }
}

impl Stderr {
    pub fn lock(&self) -> StderrLock {
        StderrLock {
            lock: self.inner.lock()
        }
    }
}

lazy_static! {
    static ref STDIN: Mutex<InputInner> = Mutex::new(InputInner {
        fd: FileDescriptor::STDIN
    });
    static ref STDOUT: Mutex<OutputInner> = Mutex::new(OutputInner {
        fd: FileDescriptor::STDOUT
    });
    static ref STDERR: Mutex<OutputInner> = Mutex::new(OutputInner {
        fd: FileDescriptor::STDOUT
    });
}

pub fn stdin() -> Stdin {
    Stdin { inner: &STDIN }
}

pub fn stdout() -> Stdout {
    Stdout { inner: &STDOUT }
}

pub fn stderr() -> Stderr {
    Stderr { inner: &STDERR }
}
