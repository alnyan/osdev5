use crate::Bpb;
use libsys::{
    stat::{Stat, OpenFlags},
    ioctl::IoctlCmd,
    error::Errno
};
use vfs::{VnodeImpl, VnodeKind, VnodeRef};

pub struct FileInode {
    pub cluster: u32,
    pub size: u32,
}

#[auto_inode]
impl VnodeImpl for FileInode {
    fn open(&mut self, _node: VnodeRef, flags: OpenFlags) -> Result<usize, Errno> {
        if flags & OpenFlags::O_ACCESS != OpenFlags::O_RDONLY {
            return Err(Errno::ReadOnly);
        }
        Ok(0)
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
}
