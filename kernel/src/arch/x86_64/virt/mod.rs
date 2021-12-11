use crate::mem::virt::table::MapAttributes;
use libsys::error::Errno;

mod table;
mod fixed;
pub use table::{EntryImpl, SpaceImpl};

bitflags! {
    pub struct RawAttributesImpl: u64 {
    }
}

impl From<MapAttributes> for RawAttributesImpl {
    fn from(src: MapAttributes) -> Self {
        todo!()
    }
}

pub unsafe fn enable() {
    todo!()
}

pub fn map_device_memory(phys: usize, count: usize) -> Result<usize, Errno> {
    todo!()
}
