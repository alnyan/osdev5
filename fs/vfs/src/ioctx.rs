use crate::{FileMode, VnodeRef};
use error::Errno;
use libcommon::{path_component_left, path_component_right};

pub struct Ioctx {
    root: VnodeRef,
    cwd: VnodeRef,
}

impl Ioctx {
    pub fn new(root: VnodeRef) -> Self {
        Self {
            cwd: root.clone(),
            root,
        }
    }

    fn _find(&self, mut at: VnodeRef, path: &str) -> Result<VnodeRef, Errno> {
        let mut element;
        let mut rest = path;

        loop {
            (element, rest) = path_component_left(rest);

            if !at.is_directory() {
                return Err(Errno::NotADirectory);
            }

            match element {
                ".." => {
                    at = at.parent();
                }
                "." => {}
                _ => break,
            }
        }

        if element.is_empty() && rest.is_empty() {
            return Ok(at);
        }
        assert!(!element.is_empty());

        let node = at.lookup(element).ok_or(Errno::DoesNotExist)?;

        if rest.is_empty() {
            Ok(node)
        } else {
            self._find(node, rest)
        }
    }

    pub fn find(&self, at: Option<VnodeRef>, mut path: &str) -> Result<VnodeRef, Errno> {
        let at = if path.starts_with('/') {
            path = path.trim_start_matches('/');
            self.root.clone()
        } else if let Some(at) = at {
            at
        } else {
            self.cwd.clone()
        };

        self._find(at, path)
    }

    pub fn mkdir(&self, at: Option<VnodeRef>, path: &str, mode: FileMode) -> Result<(), Errno> {
        let (parent, name) = path_component_right(path);
        let parent_node = self.find(at, parent)?;

        parent_node.mkdir(name, mode).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{node::VnodeData, Filesystem, Vnode, VnodeImpl, VnodeKind};
    use alloc::{boxed::Box, rc::Rc};

    pub struct DummyInode;
    pub struct DummyFs;

    impl VnodeImpl for DummyInode {
        fn create(&mut self, _at: VnodeRef, _node: VnodeRef) -> Result<(), Errno> {
            Err(Errno::NotImplemented)
        }

        fn remove(&mut self, _at: VnodeRef, _name: &str) -> Result<(), Errno> {
            Err(Errno::NotImplemented)
        }

        fn open(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
            Err(Errno::NotImplemented)
        }

        fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
            Err(Errno::NotImplemented)
        }

        fn read(&mut self, _node: VnodeRef, _pos: usize, _data: &mut [u8]) -> Result<usize, Errno> {
            Err(Errno::NotImplemented)
        }

        fn write(&mut self, _node: VnodeRef, _pos: usize, _data: &[u8]) -> Result<usize, Errno> {
            Err(Errno::NotImplemented)
        }

        fn truncate(&mut self, _node: VnodeRef, _size: usize) -> Result<(), Errno> {
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
    fn test_find_existing_absolute() {
        let root = Vnode::new("", VnodeKind::Directory);
        let d0 = Vnode::new("dir0", VnodeKind::Directory);
        let d1 = Vnode::new("dir1", VnodeKind::Directory);
        let d0d0 = Vnode::new("dir0", VnodeKind::Directory);
        let d0f0 = Vnode::new("file0", VnodeKind::Regular);
        let d1f0 = Vnode::new("file0", VnodeKind::Regular);

        root.attach(d0.clone());
        root.attach(d1.clone());
        d0.attach(d0d0.clone());
        d0.attach(d0f0.clone());
        d1.attach(d1f0.clone());

        let ioctx = Ioctx::new(root.clone());

        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/").unwrap()));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/.").unwrap()));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/./.").unwrap()));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/.///.").unwrap()));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/..").unwrap()));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/../").unwrap()));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/../.").unwrap()));
        assert!(Rc::ptr_eq(&root, &ioctx.find(None, "/../..").unwrap()));

        assert!(Rc::ptr_eq(&d0, &ioctx.find(None, "/dir0").unwrap()));
        assert!(Rc::ptr_eq(&d1, &ioctx.find(None, "/dir1").unwrap()));
        assert!(Rc::ptr_eq(&d0, &ioctx.find(None, "/dir1/../dir0").unwrap()));
        assert!(Rc::ptr_eq(
            &d1,
            &ioctx.find(None, "/dir1/../dir0/./../../.././dir1").unwrap()
        ));

        assert!(Rc::ptr_eq(&d0d0, &ioctx.find(None, "/dir0/dir0").unwrap()));
        assert!(Rc::ptr_eq(
            &d0d0,
            &ioctx.find(None, "/dir0/dir0/.").unwrap()
        ));
        assert!(Rc::ptr_eq(&d0, &ioctx.find(None, "/dir0/dir0/..").unwrap()));
        assert!(Rc::ptr_eq(
            &d0,
            &ioctx.find(None, "/dir0/dir0/../").unwrap()
        ));
        assert!(Rc::ptr_eq(
            &d0,
            &ioctx.find(None, "/dir0/dir0/../.").unwrap()
        ));

        assert!(Rc::ptr_eq(&d0f0, &ioctx.find(None, "/dir0/file0").unwrap()));
        assert!(Rc::ptr_eq(
            &d0f0,
            &ioctx.find(None, "/dir1/../dir0/./file0").unwrap()
        ));
    }

    #[test]
    fn test_find_rejects_file_dots() {
        let root = Vnode::new("", VnodeKind::Directory);
        let d0 = Vnode::new("dir0", VnodeKind::Directory);
        let d0f0 = Vnode::new("file0", VnodeKind::Regular);

        root.attach(d0.clone());
        d0.attach(d0f0.clone());

        let ioctx = Ioctx::new(root.clone());

        assert_eq!(
            ioctx.find(None, "/dir0/file0/.").unwrap_err(),
            Errno::NotADirectory
        );
        assert_eq!(
            ioctx.find(None, "/dir0/file0/..").unwrap_err(),
            Errno::NotADirectory
        );

        // TODO handle this case
        // assert_eq!(ioctx.find(None, "/dir0/file0/").unwrap_err(), Errno::NotADirectory);
    }

    #[test]
    fn test_mkdir() {
        let fs = Rc::new(DummyFs {});
        let root = Vnode::new("", VnodeKind::Directory);
        let ioctx = Ioctx::new(root.clone());

        root.set_data(VnodeData {
            node: Box::new(DummyInode {}),
            fs: fs.clone(),
        });

        assert!(ioctx.mkdir(None, "/dir0", FileMode::default_dir()).is_ok());
        assert_eq!(
            ioctx
                .mkdir(None, "/dir0", FileMode::default_dir())
                .unwrap_err(),
            Errno::AlreadyExists
        );
    }
}
