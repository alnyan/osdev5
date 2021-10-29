use crate::{Bpb, FileInode};
use alloc::{borrow::ToOwned, boxed::Box, string::String};
use error::Errno;
use libcommon::{read_le16, read_le32};
use vfs::{BlockDevice, Vnode, VnodeImpl, VnodeKind, VnodeRef};

pub struct DirectoryInode {
    pub cluster: u32,
}

pub struct FatIterator<'a> {
    dev: &'a dyn BlockDevice,
    sector: u32,
    sector_off: usize,
    len: u32,
    lfn: [u8; 128],
    lfn_len: u8,
    buf: [u8; 512],
}

#[derive(Debug)]
pub struct Dirent {
    pub name: String,
    pub size: u32,
    pub attrs: u8,
    pub cluster: u32,
}

impl VnodeImpl for DirectoryInode {
    fn create(
        &mut self,
        _parent: VnodeRef,
        _name: &str,
        _kind: VnodeKind,
    ) -> Result<VnodeRef, Errno> {
        todo!()
    }

    fn remove(&mut self, _parent: VnodeRef, _name: &str) -> Result<(), Errno> {
        todo!()
    }

    fn lookup(&mut self, parent: VnodeRef, name: &str) -> Result<VnodeRef, Errno> {
        let fs = parent.fs().unwrap();
        let dirent = {
            let dev = fs.clone().dev().unwrap();
            let fs_data = fs.data();
            let bpb: &Bpb = fs_data.as_ref().and_then(|e| e.downcast_ref()).unwrap();
            let sector = bpb.cluster_base_sector(self.cluster);

            FatIterator::new(dev, sector, bpb.sectors_per_cluster())
                .find(|ent| ent.name == name)
                .ok_or(Errno::DoesNotExist)
        }?;

        let kind = if dirent.attrs & 0x10 != 0 {
            VnodeKind::Directory
        } else {
            VnodeKind::Regular
        };

        let vnode = Vnode::new(&dirent.name, kind, Vnode::SEEKABLE);
        if kind == VnodeKind::Directory {
            vnode.set_data(Box::new(DirectoryInode {
                cluster: dirent.cluster,
            }));
        } else {
            vnode.set_data(Box::new(FileInode {
                cluster: dirent.cluster,
                size: dirent.size,
            }));
        }
        Ok(vnode)
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

impl Iterator for FatIterator<'_> {
    type Item = Dirent;

    fn next(&mut self) -> Option<Dirent> {
        loop {
            if self.len == 0 {
                return None;
            }

            if self.sector_off == 0 {
                self.dev
                    .read(self.sector as usize * 512, &mut self.buf)
                    .unwrap();
            }

            while self.sector_off < 512 {
                let off = self.sector_off;
                if self.buf[off] == 0 {
                    self.len = 0;
                    return None;
                }
                self.sector_off += 32;

                // Check for LFN entries
                if self.buf[off + 11] == 0x0F {
                    let lfn_order = self.buf[off];
                    let lfn_index = (lfn_order & 0x3F) as usize;
                    assert!(lfn_index > 0);
                    let mut lfn8 = [0u8; 13];

                    for j in 0..5 {
                        lfn8[j] = self.buf[off + 1 + j * 2];
                    }
                    for j in 0..6 {
                        lfn8[j + 5] = self.buf[off + 14 + j * 2];
                    }
                    for j in 0..2 {
                        lfn8[j + 11] = self.buf[off + 28 + j * 2];
                    }

                    let len = lfn8.iter().position(|&c| c == 0).unwrap_or(13);
                    let off = (lfn_index - 1) * 13;

                    if lfn_order & 0x40 != 0 {
                        // Last entry
                        self.lfn_len = (off + len) as u8;
                    } else {
                        assert_eq!(len, 13);
                    }
                    self.lfn[off..off + len].copy_from_slice(&lfn8[..len]);
                } else {
                    let size = read_le32(&self.buf[off + 28..]);
                    let attrs = self.buf[off + 11];
                    let cluster = ((read_le16(&self.buf[off + 20..]) as u32) << 16)
                        | (read_le16(&self.buf[off + 26..]) as u32);

                    if self.lfn_len != 0 {
                        let len = self.lfn_len as usize;
                        self.lfn_len = 0;
                        return Some(Dirent {
                            name: core::str::from_utf8(&self.lfn[..len as usize])
                                .unwrap()
                                .to_owned(),
                            attrs,
                            size,
                            cluster,
                        });
                    } else {
                        let len = self.buf[off..off + 11]
                            .iter()
                            .position(|&c| (c == 0) || (c == b' '))
                            .unwrap_or(11);
                        let name =
                            core::str::from_utf8(&self.buf[off..off + core::cmp::min(len, 8)])
                                .unwrap()
                                .to_owned();
                        let ext = if len > 8 {
                            ".".to_owned()
                                + core::str::from_utf8(&self.buf[off + 8..off + len]).unwrap()
                        } else {
                            "".to_owned()
                        };

                        return Some(Dirent {
                            name: name + &ext,
                            attrs,
                            size,
                            cluster,
                        });
                    }
                }
            }

            self.sector_off = 0;
            self.len -= 1;
            self.sector += 1;
        }
    }
}

impl FatIterator<'_> {
    pub fn new(dev: &'static dyn BlockDevice, sector: u32, sectors_per_cluster: u8) -> Self {
        Self {
            dev,
            sector,
            len: sectors_per_cluster as u32,
            sector_off: 0,
            lfn_len: 0,
            lfn: [0; 128],
            buf: [0; 512],
        }
    }
}
