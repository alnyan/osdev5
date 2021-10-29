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
use core::cell::{RefCell, Ref};
use error::Errno;
use libcommon::*;
use vfs::{Filesystem, Vnode, VnodeImpl, VnodeKind, VnodeRef, BlockDevice, FileMode};
use core::any::Any;

pub mod block;
pub use block::{BlockAllocator, BlockRef};
pub mod bvec;
use bvec::Bvec;
pub mod tar;
use tar::TarIterator;

pub struct Ramfs<A: BlockAllocator + Copy + 'static> {
    root: RefCell<Option<VnodeRef>>,
    alloc: A,
}

pub struct FileInode<'a, A: BlockAllocator + Copy + 'static> {
    data: Bvec<'a, A>,
}

pub struct DirInode;

impl<'a, A: BlockAllocator + Copy + 'static> VnodeImpl for FileInode<'a, A> {
    fn create(
        &mut self,
        _parent: VnodeRef,
        _name: &str,
        _kind: VnodeKind,
    ) -> Result<VnodeRef, Errno> {
        panic!()
    }

    fn lookup(&mut self, _parent: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
        panic!()
    }

    fn remove(&mut self, _parent: VnodeRef, _name: &str) -> Result<(), Errno> {
        panic!()
    }

    fn open(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
        Ok(0)
    }

    fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
        Ok(())
    }

    fn read(&mut self, _node: VnodeRef, pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
        self.data.read(pos, data)
    }

    fn write(&mut self, _node: VnodeRef, pos: usize, data: &[u8]) -> Result<usize, Errno> {
        self.data.write(pos, data)
    }

    fn truncate(&mut self, _node: VnodeRef, size: usize) -> Result<(), Errno> {
        self.data.resize((size + 4095) / 4096)
    }

    fn size(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
        Ok(self.data.size())
    }
}

impl VnodeImpl for DirInode {
    fn create(
        &mut self,
        _parent: VnodeRef,
        _name: &str,
        _kind: VnodeKind,
    ) -> Result<VnodeRef, Errno> {
        todo!()
    }

    fn lookup(&mut self, _parent: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
        panic!()
    }

    fn remove(&mut self, _parent: VnodeRef, _name: &str) -> Result<(), Errno> {
        Ok(())
    }

    fn open(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
        todo!()
    }

    fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
        todo!()
    }

    fn read(&mut self, _node: VnodeRef, _pos: usize, _data: &mut [u8]) -> Result<usize, Errno> {
        todo!()
    }

    fn write(&mut self, _node: VnodeRef, _pos: usize, _data: &[u8]) -> Result<usize, Errno> {
        todo!()
    }

    fn truncate(&mut self, _node: VnodeRef, _size: usize) -> Result<(), Errno> {
        todo!()
    }

    fn size(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
        todo!()
    }
}

impl<A: BlockAllocator + Copy + 'static> Filesystem for Ramfs<A> {
    fn root(self: Rc<Self>) -> Result<VnodeRef, Errno> {
        self.root.borrow().clone().ok_or(Errno::DoesNotExist)
    }

    fn data(&self) -> Option<Ref<dyn Any>> {
        None
    }

    fn dev(self: Rc<Self>) -> Option<&'static dyn BlockDevice> {
        None
    }

    // fn create_node(self: Rc<Self>, name: &str, kind: VnodeKind) -> Result<VnodeRef, Errno> {
    //     let node = Vnode::new(name, kind, Vnode::SEEKABLE);
    //     let data: Box<dyn VnodeImpl> = match kind {
    //         VnodeKind::Regular => Box::new(FileInode {
    //             data: Bvec::new(self.alloc),
    //         }),
    //         VnodeKind::Directory => Box::new(DirInode {}),
    //     };
    //     node.set_data(VnodeData {
    //         fs: self,
    //         node: data,
    //     });
    //     Ok(node)
    // }
}

impl<A: BlockAllocator + Copy + 'static> Ramfs<A> {
    pub unsafe fn open(base: *const u8, size: usize, alloc: A) -> Result<Rc<Self>, Errno> {
        let res = Rc::new(Self {
            root: RefCell::new(None),
            alloc,
        });
        *res.root.borrow_mut() = Some(res.clone().load_tar(base, size)?);
        Ok(res)
    }

    fn create_node_initial(self: Rc<Self>, name: &str, kind: VnodeKind) -> VnodeRef {
        let node = Vnode::new(name, kind, Vnode::SEEKABLE);
        node.set_fs(self.clone());
        match kind {
            VnodeKind::Directory => node.set_data(Box::new(DirInode {})),
            VnodeKind::Regular => {}
        };
        node
    }

    fn make_path(
        self: Rc<Self>,
        at: VnodeRef,
        path: &str,
        do_create: bool,
    ) -> Result<VnodeRef, Errno> {
        if path.is_empty() {
            return Ok(at);
        }
        let (element, rest) = path_component_left(path);
        assert!(!element.is_empty());

        let node = at.lookup(element);
        let node = match node {
            Some(node) => node,
            None => {
                if !do_create {
                    return Err(Errno::DoesNotExist);
                }
                // TODO file modes
                let node = at.mkdir(element, FileMode::default_dir())?;
                node
            }
        };

        if rest.is_empty() {
            Ok(node)
        } else {
            self.make_path(node, rest, do_create)
        }
    }

    unsafe fn load_tar(self: Rc<Self>, base: *const u8, size: usize) -> Result<VnodeRef, Errno> {
        let root = self.clone().create_node_initial("", VnodeKind::Directory);

        // 1. Create all the paths in TAR
        for block in TarIterator::new(base, base.add(size)) {
            let (dirname, basename) = path_component_right(block.path()?);

            let parent = self.clone().make_path(root.clone(), dirname, true)?;
            let node = self
                .clone()
                .create_node_initial(basename, block.node_kind());
            assert_eq!(node.kind(), block.node_kind());
            parent.attach(node);
        }

        // 2. Setup data blocks
        for block in TarIterator::new(base, base.add(size)) {
            if block.is_file() {
                // Will not create any dirs
                let node = self.clone().make_path(root.clone(), block.path()?, false)?;
                assert_eq!(node.kind(), block.node_kind());

                #[cfg(feature = "cow")]
                {
                    let data = block.data();
                    node.set_data(Box::new(FileInode {
                        data: Bvec::new_copy_on_write(self.alloc, data.as_ptr(), data.len()),
                    }));
                }
                #[cfg(not(feature = "cow"))]
                {
                    node.set_data(Box::new(FileInode {
                        data: Bvec::new(self.alloc)
                    }));

                    let size = block.size();
                    node.truncate(size)?;
                    if node.write(0, block.data())? != size {
                        return Err(Errno::InvalidArgument);
                    }
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
    use libcommon::Read;
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
        let fs = unsafe { Ramfs::open(data.as_ptr(), data.bytes().len(), A {}).unwrap() };

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
