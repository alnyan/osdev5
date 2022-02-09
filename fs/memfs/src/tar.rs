use libsys::{error::Errno, stat::FileMode};
use vfs::VnodeCreateKind;

#[repr(packed)]
#[allow(dead_code)]
pub struct Tar {
    name: [u8; 100],
    mode: [u8; 8],
    uid: [u8; 8],
    gid: [u8; 8],
    size: [u8; 12],
    mtime: [u8; 12],
    checksum: [u8; 8],
    type_: u8,
    link_name: [u8; 100],
    magic: [u8; 8],
    user: [u8; 32],
    group: [u8; 32],
    dev_major: [u8; 8],
    dev_minor: [u8; 8],
    prefix: [u8; 155],
}

pub struct TarIterator {
    address: *const u8,
    limit: *const u8,
    zero_blocks: usize,
}

impl TarIterator {
    pub const fn new(address: *const u8, limit: *const u8) -> Self {
        Self {
            address,
            limit,
            zero_blocks: 0,
        }
    }
}

impl Iterator for TarIterator {
    type Item = &'static Tar;

    fn next(&mut self) -> Option<Self::Item> {
        if self.address >= self.limit || self.zero_blocks == 2 {
            return None;
        }

        let bytes: &[u8; 512] = unsafe { (self.address as *const [u8; 512]).as_ref() }.unwrap();
        if bytes.iter().all(|&x| x == 0) {
            self.zero_blocks += 1;
            self.address = unsafe { self.address.add(512) };
            self.next()
        } else {
            let block: &Tar = unsafe { (self.address as *const Tar).as_ref() }.unwrap();
            self.zero_blocks = 0;
            self.address = unsafe { self.address.add(512 + align_up(block.size())) };
            Some(block)
        }
    }
}

impl Tar {
    pub fn is_file(&self) -> bool {
        self.type_ == 0 || self.type_ == b'0'
    }

    pub fn size(&self) -> usize {
        from_octal(&self.size)
    }

    pub fn path(&self) -> Result<&str, Errno> {
        let zero_index = self.name.iter().position(|&c| c == 0).unwrap();
        core::str::from_utf8(&self.name[..zero_index]).map_err(|_| Errno::InvalidArgument)
    }

    pub fn node_create_kind(&self) -> VnodeCreateKind {
        match self.type_ {
            0 | b'0' => VnodeCreateKind::File,
            b'5' => VnodeCreateKind::Directory,
            p => panic!("Unrecognized tar entry type: '{}'", p as char),
        }
    }

    pub fn mode(&self) -> FileMode {
        let t = match self.node_create_kind() {
            VnodeCreateKind::File => FileMode::S_IFREG,
            VnodeCreateKind::Directory => FileMode::S_IFDIR,
        };
        FileMode::from_bits(from_octal(&self.mode) as u32).unwrap() | t
    }

    pub fn data(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                ((self as *const _ as usize) + 512) as *const _,
                self.size(),
            )
        }
    }
}

fn from_octal(oct: &[u8]) -> usize {
    let mut res = 0usize;
    for &byte in oct {
        if byte == 0 {
            break;
        }

        res <<= 3;
        res |= (byte - b'0') as usize;
    }
    res
}

const fn align_up(size: usize) -> usize {
    (size + 511) & !511
}
