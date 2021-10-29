#![no_std]

#[cfg(test)]
#[macro_use]
extern crate std;

extern crate alloc;

use alloc::{boxed::Box, rc::Rc};
use core::any::Any;
use core::cell::{Ref, RefCell};
use error::Errno;
use libcommon::read_le32;
use vfs::{BlockDevice, Filesystem, Vnode, VnodeKind, VnodeRef};

pub mod dir;
pub use dir::{DirectoryInode, Dirent as FatEntry, FatIterator};
pub mod file;
pub use file::FileInode;
pub mod data;
pub use data::Bpb;

pub struct Fat32 {
    bpb: RefCell<Bpb>,
    root: RefCell<Option<VnodeRef>>,
    dev: &'static dyn BlockDevice,
}

impl Filesystem for Fat32 {
    fn root(self: Rc<Self>) -> Result<VnodeRef, Errno> {
        self.root.borrow().clone().ok_or(Errno::DoesNotExist)
    }

    fn dev(self: Rc<Self>) -> Option<&'static dyn BlockDevice> {
        Some(self.dev)
    }

    fn data(&self) -> Option<Ref<dyn Any>> {
        Some(self.bpb.borrow())
    }
}

impl Fat32 {
    pub fn open(dev: &'static dyn BlockDevice) -> Result<Rc<Self>, Errno> {
        let mut buf = [0u8; 512];

        dev.read(0, &mut buf)?;

        if buf[0x42] != 0x28 && buf[0x42] != 0x29 {
            panic!("Not a FAT32");
        }

        let root_cluster = read_le32(&buf[44..]);

        let res = Rc::new(Self {
            bpb: RefCell::new(Bpb::from_sector(&buf)),
            dev,
            root: RefCell::new(None),
        });

        let root = Vnode::new("", VnodeKind::Directory, Vnode::SEEKABLE);
        root.set_fs(res.clone());
        root.set_data(Box::new(DirectoryInode {
            cluster: root_cluster,
        }));
        *res.root.borrow_mut() = Some(root);

        Ok(res)
    }
}
