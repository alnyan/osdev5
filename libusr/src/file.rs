use crate::io::{AsRawFd, Error};
use crate::os;
use crate::trace;
use libsys::stat::FileDescriptor;

pub struct File {
    fd: FileDescriptor,
}

impl File {
    pub fn open(path: &str) -> Result<File, Error> {
        todo!()
    }
}

impl AsRawFd for File {
    fn as_raw_fd(&self) -> FileDescriptor {
        self.fd
    }
}

impl Drop for File {
    fn drop(&mut self) {
        todo!();
    }
}
