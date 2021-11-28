//! Kernel filesystem facilities
use crate::mem::{
    self,
    phys::{self, PageUsage},
};
use libsys::{error::Errno, stat::MountOptions};
use vfs::VnodeRef;
use memfs::BlockAllocator;

pub mod devfs;

/// Allocator implementation for memfs
#[derive(Clone, Copy)]
pub struct MemfsBlockAlloc;

unsafe impl BlockAllocator for MemfsBlockAlloc {
    fn alloc(&self) -> *mut u8 {
        if let Ok(page) = phys::alloc_page(PageUsage::Filesystem) {
            mem::virtualize(page) as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, data: *mut u8) {
        let phys = (data as usize) - mem::KERNEL_OFFSET;
        phys::free_page(phys).unwrap();
    }
}

pub fn create_filesystem(options: &MountOptions) -> Result<VnodeRef, Errno> {
    let fs_name = options.fs.unwrap();

    if fs_name == "devfs" {
        Ok(devfs::root().clone())
    } else {
        todo!();
    }
}
