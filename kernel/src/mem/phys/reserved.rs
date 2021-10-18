use crate::mem::{kernel_end_phys, PAGE_SIZE};
use core::mem::MaybeUninit;
use core::ptr::null_mut;

pub struct ReservedRegion {
    pub start: usize,
    pub end: usize,
    next: *mut ReservedRegion,
}
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
    pub const fn new(start: usize, end: usize) -> ReservedRegion {
        //assert!(start.is_paligned() && end.is_paligned());
        ReservedRegion {
            start,
            end,
            next: null_mut(),
        }
    }
}
static mut RESERVED_REGIONS_HEAD: *mut ReservedRegion = null_mut();
static mut RESERVED_REGION_KERNEL: MaybeUninit<ReservedRegion> = MaybeUninit::uninit();
static mut RESERVED_REGION_PAGES: MaybeUninit<ReservedRegion> = MaybeUninit::uninit();
pub unsafe fn reserve(usage: &str, region: *mut ReservedRegion) {
    debugln!(
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
