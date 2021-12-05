//! Device list pseudo-filesystem
use crate::util::InitOnce;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicUsize, Ordering};
use libsys::{stat::FileMode, error::Errno};
use vfs::{CharDevice, CharDeviceWrapper, Vnode, VnodeKind, VnodeRef};

/// Possible character device kinds
#[derive(Debug)]
pub enum CharDeviceType {
    /// Serial TTY (ttyS*)
    TtySerial,
}

static DEVFS_ROOT: InitOnce<VnodeRef> = InitOnce::new();

/// Initializes devfs
pub fn init() {
    let node = Vnode::new("", VnodeKind::Directory, Vnode::CACHE_READDIR | Vnode::CACHE_STAT);
    node.props_mut().mode = FileMode::default_dir();
    DEVFS_ROOT.init(node);
}

/// Returns devfs root node reference
pub fn root() -> &'static VnodeRef {
    DEVFS_ROOT.get()
}

pub fn add_named_char_device(dev: &'static dyn CharDevice, name: &str) -> Result<(), Errno> {
    infoln!("Add char device: {}", name);

    let node = Vnode::new(name, VnodeKind::Char, Vnode::CACHE_STAT);
    node.props_mut().mode = FileMode::from_bits(0o600).unwrap() | FileMode::S_IFCHR;
    node.set_data(Box::new(CharDeviceWrapper::new(dev)));

    DEVFS_ROOT.get().attach(node);

    Ok(())
}

/// Adds a character device node to the filesystem
pub fn add_char_device(dev: &'static dyn CharDevice, kind: CharDeviceType) -> Result<(), Errno> {
    static TTYS_COUNT: AtomicUsize = AtomicUsize::new(0);
    let mut buf = [0u8; 32];

    let (count, prefix) = match kind {
        CharDeviceType::TtySerial => (&TTYS_COUNT, b"ttyS"),
    };

    let value = count.fetch_add(1, Ordering::Relaxed);
    if value > 9 {
        panic!("Too many character devices of type {:?}", kind);
    }
    buf[..prefix.len()].copy_from_slice(prefix);
    buf[prefix.len()] = (value as u8) + b'0';

    let name = core::str::from_utf8(&buf[..=prefix.len()]).map_err(|_| Errno::InvalidArgument)?;

    add_named_char_device(dev, name)
}
