use alloc::{borrow::ToOwned, boxed::Box, rc::Rc, string::String, vec::Vec};
use core::cell::{RefCell, RefMut};
use crate::{Filesystem, FileMode};
use error::Errno;

pub type VnodeRef = Rc<Vnode>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VnodeKind {
    Directory,
    Regular,
}

pub struct TreeNode {
    parent: Option<VnodeRef>,
    children: Vec<VnodeRef>,
}

pub struct VnodeData {
    // Filesystem itself
    pub fs: Rc<dyn Filesystem>,
    // Something like "inode" struct + "ops" table
    pub node: Box<dyn VnodeImpl>,
}

pub struct VnodeProps {
    mode: FileMode,
}

pub struct Vnode {
    name: String,
    tree: RefCell<TreeNode>,
    props: RefCell<VnodeProps>,

    kind: VnodeKind,

    pub data: RefCell<Option<VnodeData>>,
}

pub trait VnodeImpl {
    fn create(&mut self, at: VnodeRef, node: VnodeRef) -> Result<(), Errno>;
    fn remove(&mut self, at: VnodeRef, name: &str) -> Result<(), Errno>;
}

impl Vnode {
    pub fn new(name: &str, kind: VnodeKind) -> VnodeRef {
        Rc::new(Self {
            name: name.to_owned(),
            kind,
            props: RefCell::new(VnodeProps {
                mode: FileMode::empty(),
            }),
            tree: RefCell::new(TreeNode {
                parent: None,
                children: Vec::new(),
            }),
            data: RefCell::new(None),
        })
    }

    pub fn set_data(&self, data: VnodeData) {
        *self.data.borrow_mut() = Some(data);
    }

    pub fn data(&self) -> RefMut<Option<VnodeData>> {
        self.data.borrow_mut()
    }

    // Tree operations

    fn attach(self: &VnodeRef, child: VnodeRef) {
        let parent_clone = self.clone();
        let mut parent_borrow = self.tree.borrow_mut();
        assert!(child
            .tree
            .borrow_mut()
            .parent
            .replace(parent_clone)
            .is_none());
        parent_borrow.children.push(child);
    }

    fn detach(self: &VnodeRef) {
        let mut self_borrow = self.tree.borrow_mut();
        let parent = self_borrow.parent.take().unwrap();
        let mut parent_borrow = parent.tree.borrow_mut();
        let index = parent_borrow
            .children
            .iter()
            .position(|it| Rc::ptr_eq(&it, self))
            .unwrap();
        parent_borrow.children.remove(index);
    }

    pub fn parent(self: &VnodeRef) -> VnodeRef {
        self.tree.borrow().parent.as_ref().unwrap_or(self).clone()
    }

    pub fn lookup(self: &VnodeRef, name: &str) -> Option<VnodeRef> {
        self.tree
            .borrow()
            .children
            .iter()
            .find(|e| e.name == name)
            .cloned()
    }

    pub fn mkdir(self: &VnodeRef, name: &str, mode: FileMode) -> Result<VnodeRef, Errno> {
        if self.kind != VnodeKind::Directory {
            return Err(Errno::NotADirectory);
        }

        if let Some(ref mut data) = *self.data.borrow_mut() {
            let vnode = data.fs.clone().create_node(name, VnodeKind::Directory)?;

            vnode.props.borrow_mut().mode = mode;

            if let Err(err) = data.node.create(self.clone(), vnode.clone()) {
                if err != Errno::NotImplemented {
                    return Err(err);
                }
            }

            self.attach(vnode.clone());
            Ok(vnode)
        } else {
            Err(Errno::NotImplemented)
        }
    }

    pub fn unlink(self: &VnodeRef, name: &str) -> Result<(), Errno> {
        if self.kind != VnodeKind::Directory {
            return Err(Errno::NotADirectory);
        }

        if let Some(ref mut data) = *self.data.borrow_mut() {
            let vnode = self.lookup(name).ok_or(Errno::DoesNotExist)?;

            if let Err(err) = data.node.remove(self.clone(), name) {
                if err != Errno::NotImplemented {
                    return Err(err);
                }
            }

            vnode.detach();
            Ok(())
        } else {
            Err(Errno::NotImplemented)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Filesystem;

    pub struct DummyInode;
    pub struct DummyFs;

    impl VnodeImpl for DummyInode {
        fn create(&mut self, _at: VnodeRef, _node: VnodeRef) -> Result<(), Errno> {
            Err(Errno::NotImplemented)
        }

        fn remove(&mut self, _at: VnodeRef, _name: &str) -> Result<(), Errno> {
            Err(Errno::NotImplemented)
        }
    }

    impl Filesystem for DummyFs {
        fn root(self: Rc<Self>) -> Result<VnodeRef, Errno> {
            todo!()
        }

        fn create_node(self: Rc<Self>, name: &str, kind: VnodeKind) -> Result<VnodeRef, Errno> {
            let node = Vnode::new(name, kind);
            node.set_data(VnodeData {
                node: Box::new(DummyInode {}),
                fs: self,
            });
            Ok(node)
        }
    }

    #[test]
    fn test_parent() {
        let root = Vnode::new("", VnodeKind::Directory);
        let node = Vnode::new("dir0", VnodeKind::Directory);

        root.attach(node.clone());

        assert!(Rc::ptr_eq(&root.parent(), &root));
        assert!(Rc::ptr_eq(&node.parent(), &root));
    }

    #[test]
    fn test_mkdir_unlink() {
        let fs = Rc::new(DummyFs {});
        let root = Vnode::new("", VnodeKind::Directory);

        root.set_data(VnodeData {
            node: Box::new(DummyInode {}),
            fs: fs.clone(),
        });

        let node = root.mkdir("test", FileMode::default_dir()).unwrap();

        assert_eq!(node.props.borrow().mode, FileMode::default_dir());
        assert!(Rc::ptr_eq(&node, &root.lookup("test").unwrap()));
        assert!(node.data.borrow().is_some());

        root.unlink("test").unwrap();

        assert!(root.lookup("test").is_none());
    }

    #[test]
    fn test_lookup_attach_detach() {
        let root = Vnode::new("", VnodeKind::Directory);
        let dir0 = Vnode::new("dir0", VnodeKind::Directory);
        let dir1 = Vnode::new("dir1", VnodeKind::Directory);

        root.attach(dir0.clone());
        root.attach(dir1.clone());

        assert!(Rc::ptr_eq(&dir0, &root.lookup("dir0").unwrap()));
        assert!(Rc::ptr_eq(&dir1, &root.lookup("dir1").unwrap()));
        assert!(Rc::ptr_eq(
            &root,
            dir0.tree.borrow().parent.as_ref().unwrap()
        ));
        assert!(Rc::ptr_eq(
            &root,
            dir1.tree.borrow().parent.as_ref().unwrap()
        ));
        assert!(root.lookup("dir2").is_none());

        dir0.detach();

        assert!(Rc::ptr_eq(&dir1, &root.lookup("dir1").unwrap()));
        assert!(Rc::ptr_eq(
            &root,
            dir1.tree.borrow().parent.as_ref().unwrap()
        ));
        assert!(dir0.tree.borrow().parent.is_none());
        assert!(root.lookup("dir0").is_none());
        assert!(root.lookup("dir2").is_none());
    }
}
