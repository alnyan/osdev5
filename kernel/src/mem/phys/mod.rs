use crate::mem::PAGE_SIZE;
use core::mem::size_of;
use error::Errno;

mod manager;
mod reserved;

use manager::{Manager, SimpleManager, MANAGER};
pub use reserved::ReservedRegion;

type ManagerImpl = SimpleManager;

const MAX_PAGES: usize = 1024 * 1024;

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum PageUsage {
    Reserved,
    Available,
    Kernel,
    KernelHeap,
    Paging,
    UserStack,
}

pub struct PageInfo {
    refcount: usize,
    usage: PageUsage,
}

#[derive(Clone)]
pub struct MemoryRegion {
    pub start: usize,
    pub end: usize,
}

#[repr(transparent)]
#[derive(Clone)]
pub struct SimpleMemoryIterator {
    inner: Option<MemoryRegion>,
}
impl SimpleMemoryIterator {
    pub const fn new(reg: MemoryRegion) -> Self {
        Self { inner: Some(reg) }
    }
}
impl Iterator for SimpleMemoryIterator {
    type Item = MemoryRegion;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.take()
    }
}

pub fn alloc_contiguous_pages(pu: PageUsage, count: usize) -> Result<usize, Errno> {
    MANAGER
        .lock()
        .as_mut()
        .unwrap()
        .alloc_contiguous_pages(pu, count)
}

pub fn alloc_page(pu: PageUsage) -> Result<usize, Errno> {
    MANAGER.lock().as_mut().unwrap().alloc_page(pu)
}

fn find_contiguous<T: Iterator<Item = MemoryRegion>>(iter: T, count: usize) -> Option<usize> {
    for region in iter {
        let mut collected = 0;
        let mut base_addr = None;
        for addr in (region.start..region.end).step_by(PAGE_SIZE) {
            if reserved::is_reserved(addr) {
                collected = 0;
                base_addr = None;
                continue;
            }
            if base_addr.is_none() {
                base_addr = Some(addr);
            }
            collected += 1;
            if collected == count {
                return base_addr;
            }
        }
    }
    None
}

pub unsafe fn init_from_iter<T: Iterator<Item = MemoryRegion> + Clone>(iter: T) {
    let mut mem_base = usize::MAX;
    for reg in iter.clone() {
        if reg.start < mem_base {
            mem_base = reg.start;
        }
    }
    debugln!("Memory base is {:#x}", mem_base);
    // Step 1. Count available memory
    let mut total_pages = 0usize;
    for reg in iter.clone() {
        total_pages += (reg.end - reg.start) / PAGE_SIZE;
    }
    // TODO maybe instead of size_of::<...> use Layout?
    let need_pages = ((total_pages * size_of::<PageInfo>()) + 0xFFF) / 0x1000;
    reserved::reserve_kernel();
    // Step 2. Allocate memory for page array
    let pages_base =
        find_contiguous(iter.clone(), need_pages).expect("Failed to allocate memory for page info");
    reserved::reserve_pages(pages_base, need_pages);
    // Step 3. Initialize the memory manager with available pages
    let mut manager = ManagerImpl::initialize(mem_base, pages_base, total_pages);
    let mut usable_pages = 0usize;
    'l0: for region in iter {
        for addr in (region.start..region.end).step_by(PAGE_SIZE) {
            if !reserved::is_reserved(addr) {
                manager.add_page(addr);
                usable_pages += 1;
                if usable_pages == MAX_PAGES {
                    break 'l0;
                }
            }
        }
    }
    debug!("{}K of usable physical memory\n", usable_pages * 4);
    *MANAGER.lock() = Some(manager);
}

pub unsafe fn init_from_region(base: usize, size: usize) {
    let iter = SimpleMemoryIterator::new(MemoryRegion {
        start: base,
        end: base + size,
    });

    init_from_iter(iter);
}
