use libsys::{
    calls::{sys_fstatat, sys_ioctl},
    stat::{FileDescriptor, Stat},
    ioctl::IoctlCmd,
    error::Errno,
    proc::Pid
};
use core::mem::size_of;
use core::fmt;

mod error;
pub use error::{Error, ErrorKind};
mod writer;
pub use writer::{_print};
mod stdio;
pub use stdio::{stderr, stdin, stdout, Stderr, Stdin, Stdout};

pub trait Read {
    fn read(&mut self, bytes: &mut [u8]) -> Result<usize, Error>;
}

pub trait Write {
    fn write(&mut self, bytes: &[u8]) -> Result<usize, Error>;
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> Result<(), Error>;
}

pub trait AsRawFd {
    fn as_raw_fd(&self) -> FileDescriptor;
}

pub fn tcgetpgrp(_fd: FileDescriptor) -> Result<Pid, Errno> {
    todo!()
}

pub fn tcsetpgrp(fd: FileDescriptor, pgid: Pid) -> Result<(), Errno> {
    sys_ioctl(fd, IoctlCmd::TtySetPgrp, &pgid as *const _ as usize, size_of::<Pid>()).map(|_| ())
}

pub fn stat(pathname: &str) -> Result<Stat, Error> {
    let mut buf = Stat::default();
    // TODO error handling
    sys_fstatat(None, pathname, &mut buf, 0).unwrap();
    Ok(buf)
}

// TODO use BufRead instead once it's implemented
pub(crate) fn read_line<'a, F: Read>(f: &mut F, buf: &'a mut [u8]) -> Result<Option<&'a str>, Error> {
    let mut pos = 0;
    loop {
        if pos == buf.len() {
            return Err(Error::from(Errno::OutOfMemory));
        }

        let count = f.read(&mut buf[pos..=pos])?;
        if count == 0 {
            if pos == 0 {
                return Ok(None);
            }
            break;
        }
        if buf[pos] == b'\n' {
            break;
        }

        pos += 1;
    }
    core::str::from_utf8(&buf[..pos]).map_err(|_| Error::from(Errno::InvalidArgument)).map(Some)
}
