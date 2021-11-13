use libsys::stat::FileDescriptor;
use crate::io;

pub struct File {
    fd: FileDescriptor
}

impl File {
    pub fn open(path: &str) -> Result<File, io::Error> {
        todo!()
    }
}
