use core::alloc::{GlobalAlloc, Layout};
use core::mem::{size_of, MaybeUninit};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};
use libsys::{
    calls::{sys_mmap, sys_munmap},
    debug::TraceLevel,
    error::Errno,
    mem::memset,
    proc::{MemoryAccess, MemoryMap},
};
use memoffset::offset_of;

use crate::trace_debug;

struct Allocator;

const BLOCK_MAGIC: u32 = 0xBADB10C0;
const BLOCK_MAGIC_MASK: u32 = 0xFFFFFFF0;
const BLOCK_ALLOC: u32 = 1 << 0;
const SMALL_ZONE_ELEM: usize = 256;
const SMALL_ZONE_SIZE: usize = 6 * 0x1000;
const MID_ZONE_ELEM: usize = 2048;
const MID_ZONE_SIZE: usize = 24 * 0x1000;
const LARGE_ZONE_ELEM: usize = 8192;
const LARGE_ZONE_SIZE: usize = 48 * 0x1000;

struct ZoneList {
    prev: *mut ZoneList,
    next: *mut ZoneList,
}

#[repr(C)]
struct Zone {
    size: usize,
    list: ZoneList,
}

#[repr(C)]
struct Block {
    prev: *mut Block,
    next: *mut Block,
    flags: u32,
    size: u32,
}

static mut SMALL_ZONE_LIST: MaybeUninit<ZoneList> = MaybeUninit::uninit();
static mut MID_ZONE_LIST: MaybeUninit<ZoneList> = MaybeUninit::uninit();
static mut LARGE_ZONE_LIST: MaybeUninit<ZoneList> = MaybeUninit::uninit();

impl ZoneList {
    fn init(&mut self) {
        self.prev = self;
        self.next = self;
    }

    unsafe fn init_uninit(list: &mut MaybeUninit<Self>) {
        list.assume_init_mut().init()
    }

    fn add(&mut self, new: *mut ZoneList) {
        let new = unsafe { &mut *new };
        let next = unsafe { &mut *self.next };

        next.prev = new;
        new.next = next;
        new.prev = self;
        self.next = new;
    }

    fn del(&mut self) {
        let prev = unsafe { &mut *self.prev };
        let next = unsafe { &mut *self.next };

        next.prev = prev;
        prev.next = next;
    }
}

impl Zone {
    fn alloc(size: usize) -> Result<*mut Self, Errno> {
        let pages = sys_mmap(
            0,
            size,
            MemoryAccess::READ | MemoryAccess::WRITE,
            MemoryMap::ANONYMOUS | MemoryMap::PRIVATE,
        )?;
        trace_debug!("Zone::alloc({}) => {:#x}", size, pages);

        let zone_ptr = pages as *mut Zone;
        let head_ptr = (pages + size_of::<Zone>()) as *mut Block;

        let zone = unsafe { &mut *zone_ptr };
        let head = unsafe { &mut *head_ptr };
        zone.list.init();
        zone.size = size - size_of::<Zone>();

        head.size = (size - (size_of::<Zone>() + size_of::<Block>())) as u32;
        head.flags = BLOCK_MAGIC;
        head.prev = null_mut();
        head.next = null_mut();

        Ok(zone)
    }

    unsafe fn free(zone: *mut Self) {
        trace_debug!("Zone::free({:p})", zone);
        sys_munmap(zone as usize, (&*zone).size + size_of::<Zone>())
            .expect("Failed to unmap heap pages");
    }

    fn get(item: *mut ZoneList) -> *mut Zone {
        ((item as usize) - offset_of!(Zone, list)) as *mut Zone
    }
}

unsafe fn zone_alloc(zone: &mut Zone, size: usize) -> *mut u8 {
    assert_eq!(size & 15, 0);

    let mut begin = ((zone as *mut _ as usize) + size_of::<Zone>()) as *mut Block;

    let mut block = begin;
    while !block.is_null() {
        let block_ref = &mut *block;
        if block_ref.flags & BLOCK_ALLOC != 0 {
            block = block_ref.next;
            continue;
        }

        if size == block_ref.size as usize {
            block_ref.flags |= BLOCK_ALLOC;
            let ptr = block.add(1) as *mut u8;
            // TODO fill with zeros
            return ptr;
        } else if block_ref.size as usize >= size + size_of::<Block>() {
            let cur_next = block_ref.next;
            let cur_next_ref = &mut *cur_next;
            let new_block = ((block as usize) + size_of::<Block>() + size) as *mut Block;
            let new_block_ref = &mut *new_block;

            if !cur_next.is_null() {
                cur_next_ref.prev = new_block;
            }
            new_block_ref.next = cur_next;
            new_block_ref.prev = block;
            new_block_ref.size = ((block_ref.size as usize) - size_of::<Block>() - size) as u32;
            new_block_ref.flags = BLOCK_MAGIC;
            block_ref.next = new_block;
            block_ref.size = size as u32;
            block_ref.flags |= BLOCK_ALLOC;

            let ptr = block.add(1) as *mut u8;
            // TODO fill with zeros
            return ptr;
        }

        block = block_ref.next;
    }

    null_mut()
}

unsafe fn alloc_from(list: &mut ZoneList, zone_size: usize, size: usize) -> *mut u8 {
    loop {
        let mut zone = list.next;
        while zone != list {
            let ptr = zone_alloc(&mut *Zone::get(zone), size);
            if !ptr.is_null() {
                return ptr;
            }
        }

        let zone = match Zone::alloc(zone_size) {
            Ok(zone) => zone,
            Err(e) => {
                trace_debug!("Zone alloc failed: {:?}", e);
                return null_mut();
            }
        };
        list.add(&mut (&mut *zone).list);
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        assert!(layout.align() < 16);
        let size = (layout.size() + 15) & !15;
        trace_debug!("alloc({:?})", layout);
        if size <= SMALL_ZONE_ELEM {
            alloc_from(SMALL_ZONE_LIST.assume_init_mut(), SMALL_ZONE_SIZE, size)
        } else if size <= MID_ZONE_ELEM {
            alloc_from(MID_ZONE_LIST.assume_init_mut(), MID_ZONE_SIZE, size)
        } else if size <= LARGE_ZONE_ELEM {
            alloc_from(LARGE_ZONE_LIST.assume_init_mut(), LARGE_ZONE_SIZE, size)
        } else {
            todo!();
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        trace_debug!("free({:p}, {:?})", ptr, layout);
        assert!(!ptr.is_null());
        let mut block = ptr.sub(size_of::<Block>()) as *mut Block;
        let mut block_ref = &mut *block;

        if block_ref.flags & BLOCK_MAGIC_MASK != BLOCK_MAGIC {
            panic!("Heap block is malformed: block={:p}, ptr={:p}", block, ptr);
        }
        if block_ref.flags & BLOCK_ALLOC == 0 {
            panic!(
                "Double free error in heap: block={:p}, ptr={:p}",
                block, ptr
            );
        }

        block_ref.flags &= !BLOCK_ALLOC;
        let prev = block_ref.prev;
        let next = block_ref.next;
        let prev_ref = &mut *prev;
        let next_ref = &mut *next;

        if !prev.is_null() && prev_ref.flags & BLOCK_ALLOC == 0 {
            block_ref.flags = 0;
            prev_ref.next = next;
            if !next.is_null() {
                next_ref.prev = prev;
            }
            prev_ref.size += (block_ref.size as usize + size_of::<Block>()) as u32;

            block = prev;
            block_ref = &mut *block;
        }

        if !next.is_null() && next_ref.flags & BLOCK_ALLOC == 0 {
            next_ref.flags = 0;
            if !next_ref.next.is_null() {
                (&mut *(next_ref.next)).prev = block;
            }
            block_ref.next = next_ref.next;
            block_ref.size += (next_ref.size as usize + size_of::<Block>()) as u32;
        }

        if block_ref.prev.is_null() && block_ref.next.is_null() {
            let zone = (block as usize - size_of::<Zone>()) as *mut Zone;
            assert_eq!((zone as usize) & 0xFFF, 0);
            (&mut *zone).list.del();
            Zone::free(zone);
        }
    }
}

#[alloc_error_handler]
fn alloc_error_handler(_layout: Layout) -> ! {
    loop {}
}

#[global_allocator]
static ALLOC: Allocator = Allocator;

pub unsafe fn init() {
    ZoneList::init_uninit(&mut SMALL_ZONE_LIST);
    ZoneList::init_uninit(&mut MID_ZONE_LIST);
    ZoneList::init_uninit(&mut LARGE_ZONE_LIST);
}
