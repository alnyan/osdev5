#![no_std]

#[cfg(test)]
#[macro_use]
extern crate std;

extern crate alloc;

use alloc::{rc::Rc, boxed::Box};
use core::any::Any;
use core::cell::{Ref, RefCell};
use error::Errno;
use libcommon::{read_le16, read_le32};
use vfs::{BlockDevice, Filesystem, Vnode, VnodeKind, VnodeRef};

pub mod dir;
pub use dir::{Dirent as FatEntry, FatIterator, DirectoryInode};
pub mod file;
pub use file::FileInode;

#[derive(Debug)]
struct Bpb {
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fat_count: u8,
    sectors_per_fat: u32,
}

pub struct Fat32 {
    bpb: RefCell<Bpb>,
    root: RefCell<Option<VnodeRef>>,
    dev: &'static dyn BlockDevice,
}

impl Bpb {
    pub const fn cluster_base_sector(&self, cluster: u32) -> u32 {
        let first_data_sector = self.reserved_sectors as u32
            + (self.fat_count as u32 * self.sectors_per_fat as u32);
        ((cluster - 2) * self.sectors_per_cluster as u32) + first_data_sector
    }
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
    fn create_node(self: Rc<Self>, name: &str, kind: VnodeKind, cluster: u32) -> VnodeRef {
        let res = Vnode::new(name, kind, Vnode::SEEKABLE);

        res.set_fs(self.clone());
        res.set_data(match kind {
            VnodeKind::Directory => Box::new(DirectoryInode { cluster }),
            _ => todo!(),
        });
        res
    }

    pub fn open(dev: &'static dyn BlockDevice) -> Result<Rc<Self>, Errno> {
        let mut buf = [0u8; 512];

        dev.read(0, &mut buf)?;

        if buf[0x42] != 0x28 && buf[0x42] != 0x29 {
            panic!("Not a FAT32");
        }

        let root_cluster = read_le32(&buf[44..]);

        let res = Rc::new(Self {
            bpb: RefCell::new(Bpb {
                fat_count: buf[16],
                reserved_sectors: read_le16(&buf[14..]),
                sectors_per_cluster: buf[13],
                sectors_per_fat: read_le32(&buf[36..]),
            }),
            dev,
            root: RefCell::new(None),
        });

        *res.root.borrow_mut() = Some(res.clone().create_node(
            "",
            VnodeKind::Directory,
            root_cluster,
        ));

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::fs::File;
    use std::io::{Read, Seek, SeekFrom};
    use libcommon::{Read as LRead};
    use vfs::Ioctx;

    struct FileBlockDevice {
        file: RefCell<Option<File>>,
    }

    impl FileBlockDevice {
        pub const fn new() -> Self {
            Self {
                file: RefCell::new(None),
            }
        }

        pub fn open(&self, path: &str) {
            *self.file.borrow_mut() = Some(File::open(path).unwrap());
        }
    }

    impl BlockDevice for FileBlockDevice {
        fn read(&self, pos: usize, buf: &mut [u8]) -> Result<(), Errno> {
            let mut borrow = self.file.borrow_mut();
            let file = borrow.as_mut().unwrap();
            file.seek(SeekFrom::Start(pos as u64)).unwrap();
            if file.read(buf).unwrap() != buf.len() {
                Err(Errno::InvalidArgument)
            } else {
                Ok(())
            }
        }

        fn write(&self, _pos: usize, _buf: &[u8]) -> Result<(), Errno> {
            todo!()
        }
    }

    unsafe impl Sync for FileBlockDevice {}

    #[test]
    fn test_fat32_open() {
        static DEV: FileBlockDevice = FileBlockDevice::new();

        DEV.open("test/test0.img");

        let fs = Fat32::open(&DEV).unwrap();
        let root = fs.root().unwrap();
        let ioctx = Ioctx::new(root);

        let mut buf = [0u8; 512];
        let node0 = ioctx.find(None, "FILENAME.TXT").unwrap();
        let mut file = node0.open().unwrap();

        let count = file.read(&mut buf).unwrap();
        assert_eq!(count, 15);
        assert_eq!(&buf[..15], b"This is a file\n");
    }
}
