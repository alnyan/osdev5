use vfs::{Vnode, VnodeKind, CharDevice, VnodeRef, CharDeviceWrapper};
use alloc::boxed::Box;
use crate::util::InitOnce;
use error::Errno;

static DEVFS_ROOT: InitOnce<VnodeRef> = InitOnce::new();

pub fn init() {
    DEVFS_ROOT.init(Vnode::new("", VnodeKind::Directory, 0));
}

pub fn root() -> &'static VnodeRef {
    DEVFS_ROOT.get()
}

pub fn add_char_device(name: &str, dev: &'static dyn CharDevice) -> Result<(), Errno> {
    debugln!("Add device: {}", name);
    let node = Vnode::new(name, VnodeKind::Char, 0);
    node.set_data(Box::new(CharDeviceWrapper::new(dev)));

    DEVFS_ROOT.get().attach(node);

    Ok(())
}
