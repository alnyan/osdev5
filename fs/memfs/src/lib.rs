#![feature(
    const_fn_trait_bound,
    const_mut_refs,
    maybe_uninit_extra,
    maybe_uninit_uninit_array
)]
#![no_std]

extern crate alloc;
#[cfg(test)]
#[macro_use]
extern crate std;

use alloc::{boxed::Box, rc::Rc};
use core::cell::RefCell;
use error::Errno;
use vfs::{node::VnodeData, Filesystem, Vnode, VnodeImpl, VnodeKind, VnodeRef};

pub mod block;
pub use block::{BlockAllocator, BlockRef};
pub mod bvec;
use bvec::Bvec;
pub mod tar;
use tar::{Tar, TarIterator};

pub struct Ramfs<A: BlockAllocator + Copy + 'static> {
    root: RefCell<Option<VnodeRef>>,
    alloc: A,
}

pub struct FileInode<'a, A: BlockAllocator + Copy + 'static> {
    data: Bvec<'a, A>,
}

pub struct DirInode;

impl<'a, A: BlockAllocator + Copy + 'static> VnodeImpl for FileInode<'a, A> {
    fn create(&mut self, _parent: VnodeRef, _node: VnodeRef) -> Result<(), Errno> {
        panic!()
    }

    fn remove(&mut self, _parent: VnodeRef, _name: &str) -> Result<(), Errno> {
        panic!()
    }

    fn open(&mut self, node: VnodeRef) -> Result<usize, Errno> {
        Ok(0)
    }

    fn close(&mut self, node: VnodeRef) -> Result<(), Errno> {
        Ok(())
    }

    fn read(&mut self, node: VnodeRef, pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
        self.data.read(pos, data)
    }

    fn write(&mut self, node: VnodeRef, pos: usize, data: &[u8]) -> Result<usize, Errno> {
        self.data.write(pos, data)
    }

    fn truncate(&mut self, _node: VnodeRef, size: usize) -> Result<(), Errno> {
        self.data.resize((size + 4095) / 4096)
    }
}

impl VnodeImpl for DirInode {
    fn create(&mut self, _parent: VnodeRef, _node: VnodeRef) -> Result<(), Errno> {
        Ok(())
    }

    fn remove(&mut self, _parent: VnodeRef, _name: &str) -> Result<(), Errno> {
        Ok(())
    }

    fn open(&mut self, node: VnodeRef) -> Result<usize, Errno> {
        todo!()
    }

    fn close(&mut self, node: VnodeRef) -> Result<(), Errno> {
        todo!()
    }

    fn read(&mut self, node: VnodeRef, pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
        todo!()
    }

    fn write(&mut self, node: VnodeRef, pos: usize, data: &[u8]) -> Result<usize, Errno> {
        todo!()
    }

    fn truncate(&mut self, _node: VnodeRef, _size: usize) -> Result<(), Errno> {
        todo!()
    }
}

impl<A: BlockAllocator + Copy + 'static> Filesystem for Ramfs<A> {
    fn root(self: Rc<Self>) -> Result<VnodeRef, Errno> {
        self.root.borrow().clone().ok_or(Errno::DoesNotExist)
    }

    fn create_node(self: Rc<Self>, name: &str, kind: VnodeKind) -> Result<VnodeRef, Errno> {
        let mut node = Vnode::new(name, kind);
        let data: Box<dyn VnodeImpl> = match kind {
            VnodeKind::Regular => Box::new(FileInode {
                data: Bvec::new(self.alloc),
            }),
            VnodeKind::Directory => Box::new(DirInode {}),
        };
        node.set_data(VnodeData {
            fs: self,
            node: data,
        });
        Ok(node)
    }
}

impl<A: BlockAllocator + Copy + 'static> Ramfs<A> {
    pub fn open(base: *const u8, size: usize, alloc: A) -> Result<Rc<Self>, Errno> {
        let mut res = Rc::new(Self {
            root: RefCell::new(None),
            alloc,
        });
        unsafe {
            *res.root.borrow_mut() = Some(res.clone().load_tar(base, size)?);
        }
        Ok(res)
    }

    fn make_path(
        self: Rc<Self>,
        at: VnodeRef,
        path: &str,
        kind: VnodeKind,
        do_create: bool,
    ) -> Result<VnodeRef, Errno> {
        let (element, rest) = vfs::util::path_component_left(path);
        assert!(!element.is_empty());

        let node_kind = if rest.is_empty() {
            kind
        } else {
            VnodeKind::Directory
        };

        let node = at.lookup(element);
        let node = match node {
            Some(node) => node,
            None => {
                if !do_create {
                    return Err(Errno::DoesNotExist);
                }
                let node = self.clone().create_node(element, node_kind)?;
                at.attach(node.clone());
                node
            }
        };

        if rest.is_empty() {
            Ok(node)
        } else {
            self.make_path(node, rest, kind, do_create)
        }
    }

    unsafe fn load_tar(self: Rc<Self>, base: *const u8, size: usize) -> Result<VnodeRef, Errno> {
        let root = self.clone().create_node("", VnodeKind::Directory)?;

        // 1. Create all the paths in TAR
        for block in TarIterator::new(base, base.add(size)) {
            let node =
                self.clone()
                    .make_path(root.clone(), block.path()?, block.node_kind(), true)?;
            assert_eq!(node.kind(), block.node_kind());
        }

        // 2. Setup data blocks
        for block in TarIterator::new(base, base.add(size)) {
            if block.is_file() {
                let size = block.size();
                let node = self.clone().make_path(
                    root.clone(),
                    block.path()?,
                    block.node_kind(),
                    false,
                )?;

                node.truncate(size).unwrap();
                let res = node.write(0, block.data()).unwrap();
                if res != size {
                    panic!("Expected to write {}B, got {}B", size, res);
                }
            }
        }

        Ok(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;
    use vfs::Ioctx;

    #[test]
    fn ramfs_open() {
        #[derive(Clone, Copy)]
        struct A;
        unsafe impl BlockAllocator for A {
            fn alloc(&self) -> *mut u8 {
                let b = Box::leak(Box::new([0; block::SIZE]));
                b.as_mut_ptr() as *mut _
            }
            unsafe fn dealloc(&self, ptr: *mut u8) {
                drop(Box::from_raw(ptr as *mut [u8; block::SIZE]));
            }
        }
        unsafe impl Sync for A {}

        let data = include_str!("../test/test1.tar");
        let fs = Ramfs::open(data.as_ptr(), data.bytes().len(), A {}).unwrap();

        let root = fs.root().unwrap();
        let ioctx = Ioctx::new(root.clone());

        assert!(Rc::ptr_eq(&ioctx.find(None, "/").unwrap(), &root));

        let node = ioctx.find(None, "/test1.txt").unwrap();
        let mut file = node.open().unwrap();
        let mut buf = [0u8; 1024];

        assert_eq!(file.read(&mut buf).unwrap(), 20);
        let s = core::str::from_utf8(&buf[..20]).unwrap();
        assert_eq!(s, "This is a test file\n");
    }
}
