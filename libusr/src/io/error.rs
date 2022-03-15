use libsys::error::Errno;

#[derive(Debug)]
pub struct Error {
    #[allow(dead_code)]
    repr: Repr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    InvalidData,
}

#[derive(Debug)]
enum Repr {
    Os(Errno),
    Simple(ErrorKind),
}

impl Error {
    pub const fn new(kind: ErrorKind) -> Self {
        Self {
            repr: Repr::Simple(kind),
        }
    }
}

impl From<Errno> for Error {
    fn from(e: Errno) -> Self {
        Self { repr: Repr::Os(e) }
    }
}
