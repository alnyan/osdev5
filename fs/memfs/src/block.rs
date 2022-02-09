use core::mem::{size_of, MaybeUninit};
use core::ops::{Deref, DerefMut};
use libsys::error::Errno;

pub const SIZE: usize = 4096;
pub const ENTRY_COUNT: usize = SIZE / size_of::<usize>();

// Should be the same as "usize" in layout
pub struct BlockRef<'a, A: BlockAllocator + Copy> {
    inner: Option<&'a mut [u8; SIZE]>,
    alloc: MaybeUninit<A>,
}

/// # Safety
///
/// This trait is unsafe to implement due to its direct memory management
pub unsafe trait BlockAllocator {
    fn alloc(&self) -> *mut u8;
    /// # Safety
    ///
    /// Unsafe: accepts arbitrary block addresses
    unsafe fn dealloc(&self, block: *mut u8);
}

impl<'a, A: BlockAllocator + Copy> BlockRef<'a, A> {
    pub fn new(alloc: A) -> Result<Self, Errno> {
        assert!(size_of::<A>() == 0);
        let ptr = alloc.alloc();
        if ptr.is_null() {
            Err(Errno::OutOfMemory)
        } else {
            Ok(unsafe { Self::from_raw(alloc, ptr) })
        }
    }

    pub fn new_indirect(alloc: A) -> Result<Self, Errno> {
        let mut res = Self::new(alloc)?;
        for it in res.as_mut_ref_array().iter_mut() {
            it.write(BlockRef::null());
        }
        Ok(res)
    }

    pub const fn null() -> Self {
        Self {
            inner: None,
            alloc: MaybeUninit::uninit(),
        }
    }

    /// # Safety
    ///
    /// Unsafe: does not perform checks on `data` pointer
    pub unsafe fn from_raw(alloc: A, data: *mut u8) -> Self {
        Self {
            inner: Some(&mut *(data as *mut _)),
            alloc: MaybeUninit::new(alloc),
        }
    }

    pub const fn is_null(&self) -> bool {
        self.inner.is_none()
    }

    pub fn as_mut_ref_array(&mut self) -> &mut [MaybeUninit<BlockRef<'a, A>>; ENTRY_COUNT] {
        assert_eq!(size_of::<Self>(), 8);
        unsafe { &mut *(self.deref_mut() as *mut _ as *mut _) }
    }

    pub fn as_ref_array(&self) -> &[MaybeUninit<BlockRef<'a, A>>; ENTRY_COUNT] {
        assert_eq!(size_of::<Self>(), 8);
        unsafe { &*(self.deref() as *const _ as *const _) }
    }
}

impl<'a, A: BlockAllocator + Copy> Drop for BlockRef<'a, A> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            unsafe {
                self.alloc
                    .assume_init_ref()
                    .dealloc(inner as *mut _ as *mut _);
            }
        }
    }
}

impl<'a, A: BlockAllocator + Copy> Deref for BlockRef<'a, A> {
    type Target = [u8; SIZE];

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().unwrap()
    }
}

impl<'a, A: BlockAllocator + Copy> DerefMut for BlockRef<'a, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::boxed::Box;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static A_COUNTER: AtomicUsize = AtomicUsize::new(0);
    #[test]
    fn block_allocator() {
        #[derive(Clone, Copy)]
        struct A;
        unsafe impl BlockAllocator for A {
            fn alloc(&self) -> *mut u8 {
                let b = Box::leak(Box::new([0; SIZE]));
                A_COUNTER.fetch_add(1, Ordering::SeqCst);
                b.as_mut_ptr() as *mut _
            }
            unsafe fn dealloc(&self, ptr: *mut u8) {
                A_COUNTER.fetch_sub(1, Ordering::SeqCst);
                drop(Box::from_raw(ptr as *mut [u8; SIZE]));
            }
        }
        const N: usize = 13;
        {
            let mut s: [MaybeUninit<BlockRef<A>>; N] = MaybeUninit::uninit_array();
            assert_eq!(A_COUNTER.load(Ordering::Acquire), 0);
            for i in 0..N {
                let mut block = BlockRef::new(A {}).unwrap();
                block.fill(1);
                s[i].write(block);
            }
            assert_eq!(A_COUNTER.load(Ordering::Acquire), N);
            for i in 0..N {
                unsafe {
                    s[i].assume_init_drop();
                }
            }
            assert_eq!(A_COUNTER.load(Ordering::Acquire), 0);
        }
    }
}
