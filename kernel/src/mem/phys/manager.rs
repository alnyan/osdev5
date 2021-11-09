use super::{PageInfo, PageUsage};
use crate::mem::{memcpy, memset, virtualize, PAGE_SIZE};
use crate::sync::IrqSafeSpinLock;
use core::mem;
use error::Errno;

pub unsafe trait Manager {
    fn alloc_page(&mut self, pu: PageUsage) -> Result<usize, Errno>;
    fn alloc_contiguous_pages(&mut self, pu: PageUsage, count: usize) -> Result<usize, Errno>;
    fn free_page(&mut self, page: usize) -> Result<(), Errno>;
    fn copy_cow_page(&mut self, src: usize) -> Result<usize, Errno>;
    fn fork_page(&mut self, src: usize) -> Result<usize, Errno>;
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
        for entry in pages.iter_mut() {
            mem::forget(mem::replace(
                entry,
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
        let page = &mut self.pages[self.page_index(addr)];
        assert!(page.refcount == 0 && page.usage == PageUsage::Reserved);
        page.usage = PageUsage::Available;
    }

    fn page_index(&self, page: usize) -> usize {
        page / PAGE_SIZE - self.base_index
    }

    fn alloc_single_index(&mut self, pu: PageUsage) -> Result<usize, Errno> {
        for index in 0..self.pages.len() {
            let page = &mut self.pages[index];
            if page.usage == PageUsage::Available {
                page.usage = pu;
                page.refcount = 1;
                return Ok(index);
            }
        }
        Err(Errno::OutOfMemory)
    }
}
unsafe impl Manager for SimpleManager {
    fn alloc_page(&mut self, pu: PageUsage) -> Result<usize, Errno> {
        self.alloc_single_index(pu)
            .map(|r| (self.base_index + r) * PAGE_SIZE)
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
    fn free_page(&mut self, addr: usize) -> Result<(), Errno> {
        let index = self.page_index(addr);
        let page = &mut self.pages[index];

        assert!(page.usage != PageUsage::Reserved && page.usage != PageUsage::Available);

        if page.refcount > 1 {
            page.refcount -= 1;
        } else {
            assert_eq!(page.refcount, 1);
            page.usage = PageUsage::Available;
            page.refcount = 0;
        }
        Ok(())
    }

    fn copy_cow_page(&mut self, src: usize) -> Result<usize, Errno> {
        let src_index = self.page_index(src);
        let page = &mut self.pages[src_index];
        let usage = page.usage;
        if usage != PageUsage::UserPrivate {
            panic!("CoW not available for non-UserPrivate pages: {:?}", usage);
        }

        if page.refcount > 1 {
            page.refcount -= 1;
            drop(page);
            let dst_index = self.alloc_single_index(usage)?;
            let dst = (self.base_index + dst_index) * PAGE_SIZE;
            unsafe {
                memcpy(virtualize(dst) as *mut u8, virtualize(src) as *mut u8, 4096);
            }
            Ok(dst)
        } else {
            assert_eq!(page.refcount, 1);
            // No additional operations needed
            Ok(src)
        }
    }

    fn fork_page(&mut self, src: usize) -> Result<usize, Errno> {
        let src_index = self.page_index(src);
        let page = &mut self.pages[src_index];
        let usage = page.usage;
        if usage != PageUsage::UserPrivate {
            todo!("Handle page types other than UserPrivate")
        } else {
            page.refcount += 1;
        }
        Ok(src)
    }
}

pub(super) static MANAGER: IrqSafeSpinLock<Option<SimpleManager>> = IrqSafeSpinLock::new(None);
