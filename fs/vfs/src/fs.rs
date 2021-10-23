use crate::{VnodeKind, VnodeRef};
use alloc::rc::Rc;
use error::Errno;

pub trait Filesystem {
    fn root(self: Rc<Self>) -> Result<VnodeRef, Errno>;
    fn create_node(self: Rc<Self>, name: &str, kind: VnodeKind) -> Result<VnodeRef, Errno>;
}
