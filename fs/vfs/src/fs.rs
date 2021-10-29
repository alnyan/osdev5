use crate::{VnodeRef, BlockDevice};
use core::cell::Ref;
use core::any::Any;
use alloc::rc::Rc;
use error::Errno;

pub trait Filesystem {
    fn root(self: Rc<Self>) -> Result<VnodeRef, Errno>;
    fn dev(self: Rc<Self>) -> Option<&'static dyn BlockDevice>;
    fn data<'a>(&'a self) -> Option<Ref<'a, dyn Any>>;
}
