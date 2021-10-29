use crate::{VnodeRef, BlockDevice};
use core::cell::Ref;
use core::any::Any;
use alloc::rc::Rc;
use error::Errno;

/// General filesystem interface
pub trait Filesystem {
    /// Returns root node of the filesystem
    fn root(self: Rc<Self>) -> Result<VnodeRef, Errno>;
    /// Returns storage device of the filesystem (if any)
    fn dev(self: Rc<Self>) -> Option<&'static dyn BlockDevice>;
    /// Returns filesystem's private data struct (if any)
    fn data<'a>(&'a self) -> Option<Ref<'a, dyn Any>>;
}
