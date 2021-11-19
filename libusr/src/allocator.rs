use core::alloc::{Layout, GlobalAlloc};
use core::sync::atomic::{AtomicUsize, Ordering};
use libsys::mem::memset;

use crate::trace;

struct Allocator;

static mut ALLOC_DATA: [u8; 65536] = [0; 65536];
static ALLOC_PTR: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        assert!(layout.align() < 16);
        let res = ALLOC_PTR.fetch_add((layout.size() + 15) & !15, Ordering::SeqCst);
        if res > 65536 {
            panic!("Out of memory");
        }
        trace!("alloc({:?}) = {:p}", layout, &ALLOC_DATA[res]);
        let res = &mut ALLOC_DATA[res] as *mut _;
        memset(res, 0, layout.size());
        res
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        trace!("free({:p}, {:?})", ptr, layout);
    }
}

#[alloc_error_handler]
fn alloc_error_handler(_layout: Layout) -> ! {
    loop {}
}

#[global_allocator]
static ALLOC: Allocator = Allocator;
