//! Physical memory management facilities

use crate::config::{ConfigKey, CONFIG};
use crate::mem::PAGE_SIZE;
use core::mem::size_of;
use error::Errno;

mod manager;
mod reserved;

use manager::{Manager, SimpleManager, MANAGER};
pub use reserved::ReservedRegion;

type ManagerImpl = SimpleManager;

const MAX_PAGES: usize = 1024 * 1024;

/// These describe what a memory page is used for
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum PageUsage {
    /// The page cannot be allocated/used
    Reserved,
    /// The page can be allocated and is unused at the moment
    Available,
    /// Kernel data page
    Kernel,
    /// Kernel heap page
    KernelHeap,
    /// Translation tables
    Paging,
    /// Userspace page
    UserPrivate,
    /// Filesystem data and blocks
    Filesystem,
}

/// Data structure representing a single physical memory page
pub struct PageInfo {
    refcount: usize,
    usage: PageUsage,
}

/// Page-aligned physical memory region
#[derive(Clone)]
pub struct MemoryRegion {
    /// Start address (page-aligned)
    pub start: usize,
    /// End address (page-aligned)
    pub end: usize,
}

/// Wrapper for single-region physical memory initialization
#[repr(transparent)]
#[derive(Clone)]
pub struct SimpleMemoryIterator {
    inner: Option<MemoryRegion>,
}
impl SimpleMemoryIterator {
    /// Constructs a new instance of [Self]
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

/// Allocates a contiguous range of `count` physical memory pages.
pub fn alloc_contiguous_pages(pu: PageUsage, count: usize) -> Result<usize, Errno> {
    MANAGER
        .lock()
        .as_mut()
        .unwrap()
        .alloc_contiguous_pages(pu, count)
}

/// Allocates a single physical memory page.
pub fn alloc_page(pu: PageUsage) -> Result<usize, Errno> {
    MANAGER.lock().as_mut().unwrap().alloc_page(pu)
}

/// Releases a single physical memory page back for further allocation.
pub unsafe fn free_page(page: usize) -> Result<(), Errno> {
    MANAGER.lock().as_mut().unwrap().free_page(page)
}

///
pub fn clone_page(src: usize) -> Result<usize, Errno> {
    MANAGER.lock().as_mut().unwrap().clone_page(src)
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

/// Initializes physical memory manager using an iterator of available
/// physical memory ranges
pub unsafe fn init_from_iter<T: Iterator<Item = MemoryRegion> + Clone>(iter: T) {
    let mut mem_base = usize::MAX;
    for reg in iter.clone() {
        if reg.start < mem_base {
            mem_base = reg.start;
        }
    }
    infoln!("Memory base is {:#x}", mem_base);
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
    let cfg = CONFIG.lock();
    'l0: for region in iter {
        for addr in (region.start..region.end).step_by(PAGE_SIZE) {
            if !reserved::is_reserved(addr) {
                manager.add_page(addr);
                usable_pages += 1;
                if usable_pages >= cfg.get_usize(ConfigKey::MemLimit) {
                    break 'l0;
                }
            }
        }
    }
    infoln!("{}K of usable physical memory", usable_pages * 4);
    *MANAGER.lock() = Some(manager);
}

/// Initializes physical memory manager using a single memory region.
///
/// See [init_from_iter].
pub unsafe fn init_from_region(base: usize, size: usize) {
    let iter = SimpleMemoryIterator::new(MemoryRegion {
        start: base,
        end: base + size,
    });

    init_from_iter(iter);
}
