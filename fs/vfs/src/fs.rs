use crate::{BlockDevice, VnodeRef};
use alloc::rc::Rc;
use core::any::Any;
use core::cell::Ref;
use syscall::error::Errno;

/// General filesystem interface
pub trait Filesystem {
    /// Returns root node of the filesystem
    fn root(self: Rc<Self>) -> Result<VnodeRef, Errno>;
    /// Returns storage device of the filesystem (if any)
    fn dev(self: Rc<Self>) -> Option<&'static dyn BlockDevice>;
    /// Returns filesystem's private data struct (if any)
    fn data(&self) -> Option<Ref<dyn Any>>;
}
