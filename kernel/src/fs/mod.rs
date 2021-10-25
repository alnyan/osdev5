#![allow(missing_docs)]

use crate::mem::{self, phys::{self, PageUsage}};
use memfs::BlockAllocator;

#[derive(Clone, Copy)]
pub struct MemfsBlockAlloc;

unsafe impl BlockAllocator for MemfsBlockAlloc {
    fn alloc(&self) -> *mut u8 {
        if let Ok(page) = phys::alloc_page(PageUsage::Kernel) {
            mem::virtualize(page) as *mut u8
        } else {
            core::ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, data: *mut u8) {
        todo!()
    }
}
