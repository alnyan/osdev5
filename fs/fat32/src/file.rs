use vfs::{VnodeImpl, VnodeRef, VnodeKind};
use core::ffi::c_void;
use error::Errno;
use crate::Bpb;

pub struct FileInode {
    pub cluster: u32,
    pub size: u32
}

impl VnodeImpl for FileInode {
    fn create(&mut self, _at: VnodeRef, _name: &str, _kind: VnodeKind) -> Result<VnodeRef, Errno> {
        panic!()
    }

    fn remove(&mut self, _parent: VnodeRef, _name: &str) -> Result<(), Errno> {
        panic!()
    }

    fn lookup(&mut self, _parent: VnodeRef, _name: &str) -> Result<VnodeRef, Errno> {
        panic!()
    }

    fn open(&mut self, _node: VnodeRef) -> Result<usize, Errno> {
        Ok(0)
    }

    fn close(&mut self, _node: VnodeRef) -> Result<(), Errno> {
        todo!()
    }

    fn read(&mut self, node: VnodeRef, pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
        let size = self.size as usize;
        if pos >= size {
            return Ok(0);
        }

        let fs = node.fs().unwrap();
        let dev = fs.clone().dev().unwrap();
        let fs_data = fs.data();
        let bpb: &Bpb = fs_data.as_ref().and_then(|e| e.downcast_ref()).unwrap();
        let base_sector = bpb.cluster_base_sector(self.cluster);

        let mut rem = core::cmp::min(size - pos, data.len());
        let mut off = 0usize;
        let mut buf = [0; 512];

        while rem != 0 {
            let sector_index = (pos + off) / 512;
            let sector_offset = (pos + off) % 512;
            let count = core::cmp::min(rem, 512 - sector_offset);

            dev.read((base_sector as usize + sector_index) * 512, &mut buf)?;
            let src = &buf[sector_offset..sector_offset + count];
            let dst = &mut data[off..off + count];
            dst.copy_from_slice(src);

            rem -= count;
            off += count;
        }

        Ok(off)
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

    fn ioctl(&mut self, _node: VnodeRef, _cmd: u64, _value: *mut c_void) -> Result<isize, Errno> {
        todo!()
    }
}
