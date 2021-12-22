//! Physical memory management facilities

use crate::config::{ConfigKey, CONFIG};
use crate::mem::PAGE_SIZE;
use core::mem::size_of;
use libsys::error::Errno;

mod manager;
mod reserved;

use manager::{Manager, SimpleManager, MANAGER};
pub use reserved::{ReservedRegion, reserve};

type ManagerImpl = SimpleManager;

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

/// Represents counts of allocated/available pages
#[allow(missing_docs)]
#[derive(Clone, Debug)]
pub struct PageStatistics {
    pub available: usize,
    pub kernel: usize,
    pub kernel_heap: usize,
    pub paging: usize,
    pub user_private: usize,
    pub filesystem: usize,
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

#[cfg(feature = "verbose")]
fn trace_alloc(loc: &core::panic::Location, pu: PageUsage, base: usize, count: usize) {
    use crate::debug::Level;
    println!(
        Level::Debug,
        "\x1B[36;1m[phys/alloc] {}:{} {:?} {:#x}..{:#x}\x1B[0m",
        loc.file(),
        loc.line(),
        pu,
        base,
        base + count * PAGE_SIZE
    );
}

#[cfg(feature = "verbose")]
fn trace_free(loc: &core::panic::Location, page: usize) {
    use crate::debug::Level;
    println!(
        Level::Debug,
        "\x1B[36;1m[phys/free] {}:{} {:#x}..{:#x}\x1B[0m",
        loc.file(),
        loc.line(),
        page,
        page + PAGE_SIZE
    );
}

/// Allocates a contiguous range of `count` physical memory pages.
#[cfg_attr(feature = "verbose", track_caller)]
pub fn alloc_contiguous_pages(pu: PageUsage, count: usize) -> Result<usize, Errno> {
    let res = MANAGER
        .lock()
        .as_mut()
        .unwrap()
        .alloc_contiguous_pages(pu, count);
    #[cfg(feature = "verbose")]
    if let Ok(base) = res {
        trace_alloc(&core::panic::Location::caller(), pu, base, count);
    }
    res
}

/// Allocates a single physical memory page.
#[cfg_attr(feature = "verbose", track_caller)]
pub fn alloc_page(pu: PageUsage) -> Result<usize, Errno> {
    let res = MANAGER.lock().as_mut().unwrap().alloc_page(pu);
    #[cfg(feature = "verbose")]
    if let Ok(base) = res {
        trace_alloc(&core::panic::Location::caller(), pu, base, 1);
    }
    res
}

/// Releases a single physical memory page back for further allocation.
///
/// # Safety
///
/// Unsafe: accepts arbitrary `page` arguments
#[cfg_attr(feature = "verbose", track_caller)]
pub unsafe fn free_page(page: usize) -> Result<(), Errno> {
    #[cfg(feature = "verbose")]
    {
        trace_free(&core::panic::Location::caller(), page);
    }
    MANAGER.lock().as_mut().unwrap().free_page(page)
}

/// Returns current statistics for page allocation
pub fn statistics() -> PageStatistics {
    MANAGER.lock().as_ref().unwrap().statistics()
}

/// Clones the source page.
///
/// If returned address is the same as `page`, this means
/// `page`'s refcount has increased and the page is Copy-on-Write.
/// This case has to be handled accordingly
///
/// # Safety
///
/// Unsafe: accepts arbitrary `page` arguments
pub unsafe fn fork_page(page: usize) -> Result<usize, Errno> {
    MANAGER.lock().as_mut().unwrap().fork_page(page)
}

/// Copies a Copy-on-Write page. If refcount is already 1,
/// page does not need to be copied and the same address is returned.
///
/// # Safety
///
/// Unsafe: accepts arbitrary `page` arguments
pub unsafe fn copy_cow_page(page: usize) -> Result<usize, Errno> {
    MANAGER.lock().as_mut().unwrap().copy_cow_page(page)
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
///
/// # Safety
///
/// Unsafe: caller must ensure validity of passed memory regions.
/// The function may not be called twice.
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
        let upper = (reg.end - mem_base) / PAGE_SIZE;
        if upper > total_pages {
            total_pages = upper;
        }
    }
    // TODO maybe instead of size_of::<...> use Layout?
    let need_pages = ((total_pages * size_of::<PageInfo>()) + 0xFFF) / 0x1000;
    reserved::reserve_kernel();
    reserved::reserve_initrd();
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
///
/// # Safety
///
/// Unsafe: see [init_from_iter].
pub unsafe fn init_from_region(base: usize, size: usize) {
    let iter = SimpleMemoryIterator::new(MemoryRegion {
        start: base,
        end: base + size,
    });

    init_from_iter(iter);
}
