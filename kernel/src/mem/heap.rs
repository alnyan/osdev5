//! Kernel heap allocation facilities

use crate::sync::IrqSafeSpinLock;
use crate::util::InitOnce;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

struct SystemAlloc;

struct Heap {
    base: usize,
    size: usize,
    ptr: usize,
}

unsafe impl GlobalAlloc for SystemAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        HEAP.get().lock().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        HEAP.get().lock().dealloc(ptr, layout)
    }
}

impl Heap {
    unsafe fn alloc(&mut self, layout: Layout) -> *mut u8 {
        // Simple bump allocation
        assert!(layout.align() <= 16);
        let size = (layout.size() + 15) & !15;
        if self.ptr + size >= self.size {
            return null_mut();
        }

        let ptr = self.ptr;
        self.ptr += size;

        (self.base + ptr) as *mut u8
    }

    unsafe fn dealloc(&mut self, _ptr: *mut u8, _layout: Layout) {}
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("Allocation failed: {:?}", layout)
}

#[global_allocator]
static SYSTEM_ALLOC: SystemAlloc = SystemAlloc;

static HEAP: InitOnce<IrqSafeSpinLock<Heap>> = InitOnce::new();

/// Initializes kernel heap with virtual `base` address and `size`.
///
/// # Safety
///
/// Unsafe: accepts arbitrary `base` and `size` parameters.
pub unsafe fn init(base: usize, size: usize) {
    let heap = Heap { base, size, ptr: 0 };

    infoln!("Kernel heap: {:#x}..{:#x}", base, base + size);

    HEAP.init(IrqSafeSpinLock::new(heap));
}
