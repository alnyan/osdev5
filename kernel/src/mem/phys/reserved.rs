use crate::mem::{kernel_end_phys, PAGE_SIZE};
use core::mem::MaybeUninit;
use core::ptr::null_mut;

/// Data structure representing a region of unusable memory
pub struct ReservedRegion {
    /// Start address (page aligned)
    pub start: usize,
    /// End address (page aligned)
    pub end: usize,
    next: *mut ReservedRegion,
}
/// Struct for iterating over reserved memory regions
pub struct ReservedRegionIterator {
    ptr: *mut ReservedRegion,
}
impl Iterator for ReservedRegionIterator {
    type Item = &'static mut ReservedRegion;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = unsafe { self.ptr.as_mut() } {
            self.ptr = item.next;
            Some(item)
        } else {
            None
        }
    }
}
impl ReservedRegion {
    /// Constructs a new instance of [Self]
    pub const fn new(start: usize, end: usize) -> ReservedRegion {
        assert!(start & 0xFFF == 0 && end & 0xFFF == 0);
        ReservedRegion {
            start,
            end,
            next: null_mut(),
        }
    }
}
static mut RESERVED_REGIONS_HEAD: *mut ReservedRegion = null_mut();
static mut RESERVED_REGION_KERNEL: MaybeUninit<ReservedRegion> = MaybeUninit::uninit();
static mut RESERVED_REGION_INITRD: MaybeUninit<ReservedRegion> = MaybeUninit::uninit();
static mut RESERVED_REGION_PAGES: MaybeUninit<ReservedRegion> = MaybeUninit::uninit();

/// Adds a `region` to reserved memory region list.
///
/// # Safety
///
/// Unsafe: `region` is passed as a raw pointer.
pub unsafe fn reserve(usage: &str, region: *mut ReservedRegion) {
    infoln!(
        "Reserving {:?} region: {:#x}..{:#x}",
        usage,
        (*region).start,
        (*region).end
    );
    (*region).next = RESERVED_REGIONS_HEAD;
    RESERVED_REGIONS_HEAD = region;
}

pub(super) unsafe fn reserve_kernel() {
    RESERVED_REGION_KERNEL.write(ReservedRegion::new(0, kernel_end_phys()));
    reserve("kernel", RESERVED_REGION_KERNEL.as_mut_ptr());
}
pub(super) unsafe fn reserve_pages(base: usize, count: usize) {
    RESERVED_REGION_PAGES.write(ReservedRegion::new(base, base + count * PAGE_SIZE));
    reserve("pages", RESERVED_REGION_PAGES.as_mut_ptr());
}
pub(super) unsafe fn reserve_initrd() {
    use crate::config::{ConfigKey, CONFIG};
    let cfg = CONFIG.lock();
    let initrd_start = cfg.get_usize(ConfigKey::InitrdBase);
    let initrd_size = cfg.get_usize(ConfigKey::InitrdSize);
    if initrd_start != 0 {
        RESERVED_REGION_INITRD.write(ReservedRegion::new(
            initrd_start,
            (initrd_start + initrd_size + 4095) & !4095,
        ));
        reserve("initrd", RESERVED_REGION_INITRD.as_mut_ptr());
    }
}

/// Returns `true` if physical memory referred to by `page` cannot be
/// used and/or allocated
pub fn is_reserved(page: usize) -> bool {
    unsafe {
        let mut iter = RESERVED_REGIONS_HEAD;
        while !iter.is_null() {
            let region = &*iter;
            if page >= region.start && page < region.end {
                return true;
            }
            iter = region.next;
        }
    }
    false
}
