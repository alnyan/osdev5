use super::{PageInfo, PageUsage};
use crate::mem::{virtualize, PAGE_SIZE};
use crate::sync::IrqSafeNullLock;
use core::mem;
use error::Errno;

pub unsafe trait Manager {
    fn alloc_page(&mut self, pu: PageUsage) -> Result<usize, Errno>;
    fn alloc_contiguous_pages(&mut self, pu: PageUsage, count: usize) -> Result<usize, Errno>;
    fn free_page(&mut self, page: usize) -> Result<(), Errno>;
    // TODO status()
}
pub struct SimpleManager {
    pages: &'static mut [PageInfo],
    base_index: usize,
}
impl SimpleManager {
    pub(super) unsafe fn initialize(base: usize, at: usize, count: usize) -> Self {
        let pages: &'static mut [PageInfo] =
            core::slice::from_raw_parts_mut(virtualize(at) as *mut _, count);
        // Initialize uninit pages
        for index in 0..count {
            mem::forget(mem::replace(
                &mut pages[index],
                PageInfo {
                    refcount: 0,
                    usage: PageUsage::Reserved,
                },
            ));
        }
        Self {
            base_index: base / PAGE_SIZE,
            pages,
        }
    }
    pub(super) unsafe fn add_page(&mut self, addr: usize) {
        let page = &mut self.pages[addr / PAGE_SIZE - self.base_index];
        assert!(page.refcount == 0 && page.usage == PageUsage::Reserved);
        page.usage = PageUsage::Available;
    }
}
unsafe impl Manager for SimpleManager {
    fn alloc_page(&mut self, pu: PageUsage) -> Result<usize, Errno> {
        for index in 0..self.pages.len() {
            let page = &mut self.pages[index];
            if page.usage == PageUsage::Available {
                page.usage = pu;
                page.refcount = 1;
                return Ok((self.base_index + index) * PAGE_SIZE);
            }
        }
        Err(Errno::OutOfMemory)
    }
    fn alloc_contiguous_pages(&mut self, pu: PageUsage, count: usize) -> Result<usize, Errno> {
        'l0: for i in 0..self.pages.len() {
            for j in 0..count {
                if self.pages[i + j].usage != PageUsage::Available {
                    continue 'l0;
                }
            }
            for j in 0..count {
                let page = &mut self.pages[i + j];
                assert!(page.usage == PageUsage::Available);
                page.usage = pu;
                page.refcount = 1;
            }
            return Ok((self.base_index + i) * PAGE_SIZE);
        }
        Err(Errno::OutOfMemory)
    }
    fn free_page(&mut self, _page: usize) -> Result<(), Errno> {
        todo!()
    }
}

pub(super) static MANAGER: IrqSafeNullLock<Option<SimpleManager>> = IrqSafeNullLock::new(None);
