use crate::{block, BlockAllocator, BlockRef};
use core::cmp::min;
use core::mem::MaybeUninit;
use core::ops::{Index, IndexMut};
use error::Errno;

const L0_BLOCKS: usize = 32; // 128K
const L1_BLOCKS: usize = 8; // 16M

pub struct Bvec<'a, A: BlockAllocator + Copy> {
    capacity: usize,
    size: usize,
    l0: [MaybeUninit<BlockRef<'a, A>>; L0_BLOCKS],
    l1: [MaybeUninit<BlockRef<'a, A>>; L1_BLOCKS],
    l2: MaybeUninit<BlockRef<'a, A>>,
    #[cfg(feature = "cow")]
    cow_source: *const u8,
    alloc: A,
}
impl<'a, A: BlockAllocator + Copy> Bvec<'a, A> {
    pub fn new(alloc: A) -> Self {
        let mut res = Self {
            capacity: 0,
            size: 0,
            l0: MaybeUninit::uninit_array(),
            l1: MaybeUninit::uninit_array(),
            l2: MaybeUninit::uninit(),
            alloc,
            #[cfg(feature = "cow")]
            cow_source: core::ptr::null_mut(),
        };
        for it in res.l0.iter_mut() {
            it.write(BlockRef::null());
        }
        for it in res.l1.iter_mut() {
            it.write(BlockRef::null());
        }
        res.l2.write(BlockRef::null());
        res
    }

    #[cfg(feature = "cow")]
    pub fn is_cow(&self) -> bool {
        !self.cow_source.is_null()
    }

    #[cfg(feature = "cow")]
    pub unsafe fn setup_cow(&mut self, src: *const u8, size: usize) {
        self.cow_source = src;
        self.size = size;
    }

    pub const fn size(&self) -> usize {
        self.size
    }

    #[cfg(feature = "cow")]
    pub fn drop_cow(&mut self) {
        assert!(self.is_cow());
        let src_slice = unsafe { core::slice::from_raw_parts(self.cow_source, self.size) };
        self.cow_source = core::ptr::null_mut();

        self.resize((self.size + 4095) / 4096).unwrap();
        self.write(0, src_slice).unwrap();
    }

    pub fn resize(&mut self, cap: usize) -> Result<(), Errno> {
        #[cfg(feature = "cow")]
        assert!(!self.is_cow());

        if cap <= self.capacity {
            let mut curr = self.capacity;
            while curr != cap {
                curr -= 1;
                let mut index = curr;
                if index >= L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT {
                    index -= L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT;
                    let l1i = index / block::ENTRY_COUNT;
                    let l0i = index % block::ENTRY_COUNT;
                    let l2r = unsafe { self.l2.assume_init_mut() };
                    assert!(!l2r.is_null());
                    let l1r = unsafe { l2r.as_mut_ref_array()[l1i].assume_init_mut() };
                    assert!(!l1r.is_null());
                    let l0r = unsafe { l1r.as_mut_ref_array()[l0i].assume_init_mut() };
                    assert!(!l0r.is_null());
                    *l0r = BlockRef::null();
                    if l0i == 0 {
                        *l1r = BlockRef::null();
                    }
                    if index == 0 {
                        *l2r = BlockRef::null();
                    }
                    continue;
                }
                if index >= L0_BLOCKS {
                    index -= L0_BLOCKS;
                    let l1i = index / block::ENTRY_COUNT;
                    let l0i = index % block::ENTRY_COUNT;
                    let l1r = unsafe { self.l1[l1i].assume_init_mut() };
                    assert!(!l1r.is_null());
                    let l0r = unsafe { l1r.as_mut_ref_array()[l0i].assume_init_mut() };
                    assert!(!l0r.is_null());
                    *l0r = BlockRef::null();
                    if l0i == 0 {
                        *l1r = BlockRef::null();
                    }
                    continue;
                }
                let l0r = unsafe { self.l0[index].assume_init_mut() };
                assert!(!l0r.is_null());
                *l0r = BlockRef::null();
                continue;
            }
        } else {
            for mut index in self.capacity..cap {
                if index < L0_BLOCKS {
                    let l0r = unsafe { self.l0[index].assume_init_mut() };
                    assert!(l0r.is_null());
                    *l0r = BlockRef::new(self.alloc)?;
                    continue;
                }
                index -= L0_BLOCKS;
                if index < L1_BLOCKS * block::ENTRY_COUNT {
                    let l1i = index / block::ENTRY_COUNT;
                    let l0i = index % block::ENTRY_COUNT;
                    let l1r = unsafe { self.l1[l1i].assume_init_mut() };
                    if l1r.is_null() {
                        *l1r = BlockRef::new_indirect(self.alloc)?;
                    }
                    let l0r = unsafe { l1r.as_mut_ref_array()[l0i].assume_init_mut() };
                    assert!(l0r.is_null());
                    *l0r = BlockRef::new(self.alloc)?;
                    continue;
                }
                index -= L1_BLOCKS * block::ENTRY_COUNT;
                if index < block::ENTRY_COUNT * block::ENTRY_COUNT {
                    let l1i = index / block::ENTRY_COUNT;
                    let l0i = index % block::ENTRY_COUNT;
                    let l2r = unsafe { self.l2.assume_init_mut() };
                    if l2r.is_null() {
                        *l2r = BlockRef::new_indirect(self.alloc)?;
                    }
                    let l1r = unsafe { l2r.as_mut_ref_array()[l1i].assume_init_mut() };
                    if l1r.is_null() {
                        *l1r = BlockRef::new_indirect(self.alloc)?;
                    }
                    let l0r = unsafe { l1r.as_mut_ref_array()[l0i].assume_init_mut() };
                    assert!(l0r.is_null());
                    *l0r = BlockRef::new(self.alloc)?;
                    continue;
                }
                unimplemented!();
            }
        }
        self.capacity = cap;
        Ok(())
    }
    pub fn write(&mut self, mut pos: usize, data: &[u8]) -> Result<usize, Errno> {
        if pos > self.size {
            return Err(Errno::InvalidArgument);
        }

        #[cfg(feature = "cow")]
        if self.is_cow() {
            self.drop_cow();
        }

        let mut rem = data.len();
        let mut doff = 0usize;
        if pos + rem > self.size {
            self.size = pos + rem;
            self.resize((pos + rem + block::SIZE - 1) / block::SIZE)?;
        }
        while rem > 0 {
            let index = pos / block::SIZE;
            let off = pos % block::SIZE;
            let count = min(block::SIZE - off, rem);
            let block = &mut self[index];
            let dst = &mut block[off..off + count];
            let src = &data[doff..doff + count];
            dst.copy_from_slice(src);
            doff += count;
            pos += count;
            rem -= count;
        }
        Ok(doff)
    }
    pub fn read(&self, mut pos: usize, data: &mut [u8]) -> Result<usize, Errno> {
        if pos > self.size {
            return Err(Errno::InvalidArgument);
        }

        let mut rem = min(self.size - pos, data.len());

        #[cfg(feature = "cow")]
        if self.is_cow() {
            let cow_data = unsafe { core::slice::from_raw_parts(self.cow_source, self.size) };
            data[..rem].copy_from_slice(&cow_data[pos..pos + rem]);
            return Ok(rem);
        }

        let mut doff = 0usize;
        while rem > 0 {
            let index = pos / block::SIZE;
            let off = pos % block::SIZE;
            let count = min(block::SIZE - off, rem);
            let block = &self[index];
            let src = &block[off..off + count];
            let dst = &mut data[doff..doff + count];
            dst.copy_from_slice(src);
            doff += count;
            pos += count;
            rem -= count;
        }
        Ok(doff)
    }
}
impl<'a, A: BlockAllocator + Copy> Index<usize> for Bvec<'a, A> {
    type Output = BlockRef<'a, A>;
    fn index(&self, mut index: usize) -> &Self::Output {
        if index >= self.capacity {
            panic!(
                "Index exceeds bvec capacity ({} >= {})",
                index, self.capacity
            );
        }
        if index < L0_BLOCKS {
            return unsafe { self.l0[index].assume_init_ref() };
        }
        index -= L0_BLOCKS;
        if index < L1_BLOCKS * block::ENTRY_COUNT {
            return unsafe {
                let l1 = self.l1[index / block::ENTRY_COUNT].assume_init_ref();
                l1.as_ref_array()[index % block::ENTRY_COUNT].assume_init_ref()
            };
        }
        index -= L1_BLOCKS * block::ENTRY_COUNT;
        if index < block::ENTRY_COUNT * block::ENTRY_COUNT {
            return unsafe {
                let l2 = self.l2.assume_init_ref();
                let l1 = l2.as_ref_array()[index / block::ENTRY_COUNT].assume_init_ref();
                l1.as_ref_array()[index % block::ENTRY_COUNT].assume_init_ref()
            };
        }
        unimplemented!();
    }
}
impl<'a, A: BlockAllocator + Copy> IndexMut<usize> for Bvec<'a, A> {
    fn index_mut(&mut self, mut index: usize) -> &mut Self::Output {
        if index >= self.capacity {
            panic!(
                "Index exceeds bvec capacity ({} >= {})",
                index, self.capacity
            );
        }
        if index < L0_BLOCKS {
            return unsafe { self.l0[index].assume_init_mut() };
        }
        index -= L0_BLOCKS;
        if index < L1_BLOCKS * block::ENTRY_COUNT {
            return unsafe {
                let l1 = self.l1[index / block::ENTRY_COUNT].assume_init_mut();
                l1.as_mut_ref_array()[index % block::ENTRY_COUNT].assume_init_mut()
            };
        }
        index -= L1_BLOCKS * block::ENTRY_COUNT;
        if index < block::ENTRY_COUNT * block::ENTRY_COUNT {
            return unsafe {
                let l2 = self.l2.assume_init_mut();
                let l1 = l2.as_mut_ref_array()[index / block::ENTRY_COUNT].assume_init_mut();
                l1.as_mut_ref_array()[index % block::ENTRY_COUNT].assume_init_mut()
            };
        }
        unimplemented!();
    }
}
impl<'a, A: BlockAllocator + Copy> Drop for Bvec<'a, A> {
    fn drop(&mut self) {
        for i in 0..min(L0_BLOCKS, self.capacity) {
            unsafe {
                self.l0[i].assume_init_drop();
            }
        }
        if self.capacity > L0_BLOCKS {}
    }
}

#[cfg(feature = "cow")]
#[cfg(test)]
mod cow_tests {
    use super::*;
    use std::boxed::Box;

    #[derive(Clone, Copy)]
    struct TestAlloc;
    unsafe impl BlockAllocator for TestAlloc {
        fn alloc(&self) -> *mut u8 {
            let b = Box::leak(Box::new([0; block::SIZE]));
            b.as_mut_ptr() as *mut _
        }
        unsafe fn dealloc(&self, ptr: *mut u8) {
            drop(Box::from_raw(ptr as *mut [u8; block::SIZE]));
        }
    }

    #[test]
    fn bvec_write_copy_simple() {
        let mut bvec = Bvec::new(TestAlloc {});
        let mut buf = [0u8; 512];
        let source_data = b"This is initial data\n";
        unsafe {
            bvec.setup_cow(source_data.as_ptr(), source_data.len());
        }
        assert!(bvec.is_cow());
        assert_eq!(bvec.size(), source_data.len());
        assert_eq!(bvec.capacity, 0);

        bvec.write(8, b"testing").unwrap();

        assert!(!bvec.is_cow());
        assert_eq!(bvec.size(), source_data.len());
        assert_eq!(bvec.capacity, 1);

        assert_eq!(bvec.read(0, &mut buf).unwrap(), source_data.len());
        assert_eq!(&mut buf[..source_data.len()], b"This is testing data\n");
    }

    #[test]
    fn bvec_write_copy_l0() {
        let mut bvec = Bvec::new(TestAlloc {});
        let mut source_data = [0u8; 4096 * 2 - 2];
        let mut buf = [0u8; 512];
        for i in 0..source_data.len() {
            source_data[i] = (i & 0xFF) as u8;
        }
        unsafe {
            bvec.setup_cow(source_data.as_ptr(), source_data.len());
        }
        assert!(bvec.is_cow());
        assert_eq!(bvec.size(), source_data.len());
        assert_eq!(bvec.capacity, 0);

        bvec.write(0, b"test").unwrap();

        assert!(!bvec.is_cow());
        assert_eq!(bvec.size(), source_data.len());
        assert_eq!(bvec.capacity, 2);

        assert_eq!(bvec.read(0, &mut buf).unwrap(), 512);
        assert_eq!(&buf[..4], b"test");
        for i in 4..512 {
            assert_eq!(buf[i], (i & 0xFF) as u8);
        }
        assert_eq!(bvec.read(512, &mut buf).unwrap(), 512);
        for i in 0..512 {
            assert_eq!(buf[i], ((i + 512) & 0xFF) as u8);
        }

        bvec.write(source_data.len(), b"test");
        assert_eq!(bvec.size(), 4096 * 2 + 2);
        assert_eq!(bvec.capacity, 3);

        assert_eq!(bvec.read(4096 * 2, &mut buf).unwrap(), 2);
        assert_eq!(&buf[..2], b"st");
        assert_eq!(bvec.read(4096 * 2 - 2, &mut buf).unwrap(), 4);
        assert_eq!(&buf[..4], b"test");
    }
}

#[cfg(feature = "test_bvec")]
#[cfg(test)]
mod bvec_tests {
    use super::*;
    use std::boxed::Box;
    use std::mem::MaybeUninit;
    use std::sync::atomic::{AtomicUsize, Ordering};
    static A_COUNTER: AtomicUsize = AtomicUsize::new(0);
    #[derive(Clone, Copy)]
    struct TestAlloc;
    unsafe impl BlockAllocator for TestAlloc {
        fn alloc(&self) -> *mut u8 {
            let b = Box::leak(Box::new([0; block::SIZE]));
            eprintln!("alloc {:p}", b);
            b.as_mut_ptr() as *mut _
        }
        unsafe fn dealloc(&self, ptr: *mut u8) {
            eprintln!("drop {:p}", ptr);
            drop(Box::from_raw(ptr as *mut [u8; block::SIZE]));
        }
    }
    #[test]
    fn bvec_allocation() {
        #[derive(Clone, Copy)]
        struct A;
        unsafe impl BlockAllocator for A {
            fn alloc(&self) -> *mut u8 {
                let b = Box::leak(Box::new([0; block::SIZE]));
                A_COUNTER.fetch_add(1, Ordering::SeqCst);
                b.as_mut_ptr() as *mut _
            }
            unsafe fn dealloc(&self, ptr: *mut u8) {
                A_COUNTER.fetch_sub(1, Ordering::SeqCst);
                drop(Box::from_raw(ptr as *mut [u8; block::SIZE]));
            }
        }
        let mut bvec = Bvec::new(A {});
        assert_eq!(A_COUNTER.load(Ordering::Acquire), 0);
        bvec.resize(123).unwrap();
        unsafe {
            for i in 0..L0_BLOCKS {
                assert!(!bvec.l0[i].assume_init_ref().is_null());
            }
            let l1r = bvec.l1[0].assume_init_ref();
            assert!(!l1r.is_null());
            for i in 0..123 - L0_BLOCKS {
                assert!(!l1r.as_ref_array()[i].assume_init_ref().is_null());
            }
        }
        assert_eq!(A_COUNTER.load(Ordering::Acquire), 123 + 1);
        bvec.resize(123 + block::ENTRY_COUNT).unwrap();
        unsafe {
            for i in 0..L0_BLOCKS {
                assert!(!bvec.l0[i].assume_init_ref().is_null());
            }
            for i in 0..(123 + block::ENTRY_COUNT) - L0_BLOCKS {
                let l1i = i / block::ENTRY_COUNT;
                let l0i = i % block::ENTRY_COUNT;
                let l1r = bvec.l1[l1i].assume_init_ref();
                assert!(!l1r.is_null());
                assert!(!l1r.as_ref_array()[l0i].assume_init_ref().is_null());
            }
        }
        assert_eq!(
            A_COUNTER.load(Ordering::Acquire),
            123 + block::ENTRY_COUNT + 2
        );
        bvec.resize(L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT)
            .unwrap();
        unsafe {
            for i in 0..L0_BLOCKS {
                assert!(!bvec.l0[i].assume_init_ref().is_null());
            }
            for i in 0..L1_BLOCKS * block::ENTRY_COUNT {
                let l1i = i / block::ENTRY_COUNT;
                let l0i = i % block::ENTRY_COUNT;
                let l1r = bvec.l1[l1i].assume_init_ref();
                assert!(!l1r.is_null());
                assert!(!l1r.as_ref_array()[l0i].assume_init_ref().is_null());
            }
        }
        assert_eq!(
            A_COUNTER.load(Ordering::Acquire),
            L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + L1_BLOCKS
        );
        bvec.resize(L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + block::ENTRY_COUNT * 4)
            .unwrap();
        unsafe {
            for i in 0..L0_BLOCKS {
                assert!(!bvec.l0[i].assume_init_ref().is_null());
            }
            for i in 0..L1_BLOCKS * block::ENTRY_COUNT {
                let l1i = i / block::ENTRY_COUNT;
                let l0i = i % block::ENTRY_COUNT;
                let l1r = bvec.l1[l1i].assume_init_ref();
                assert!(!l1r.is_null());
                assert!(!l1r.as_ref_array()[l0i].assume_init_ref().is_null());
            }
            let l2r = bvec.l2.assume_init_ref();
            assert!(!l2r.is_null());
            for i in 0..block::ENTRY_COUNT * 4 {
                let l1i = i / block::ENTRY_COUNT;
                let l0i = i % block::ENTRY_COUNT;
                let l1r = l2r.as_ref_array()[l1i].assume_init_ref();
                assert!(!l1r.is_null());
                assert!(!l1r.as_ref_array()[l0i].assume_init_ref().is_null());
            }
        }
        assert_eq!(
            A_COUNTER.load(Ordering::Acquire),
            L0_BLOCKS + // L0
            L1_BLOCKS * block::ENTRY_COUNT + L1_BLOCKS + // L1
            block::ENTRY_COUNT * 4 + 4 + 1
        );
        bvec.resize(L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + block::ENTRY_COUNT * 3 + 1)
            .unwrap();
        unsafe {
            for i in 0..L0_BLOCKS {
                assert!(!bvec.l0[i].assume_init_ref().is_null());
            }
            for i in 0..L1_BLOCKS * block::ENTRY_COUNT {
                let l1i = i / block::ENTRY_COUNT;
                let l0i = i % block::ENTRY_COUNT;
                let l1r = bvec.l1[l1i].assume_init_ref();
                assert!(!l1r.is_null());
                assert!(!l1r.as_ref_array()[l0i].assume_init_ref().is_null());
            }
            let l2r = bvec.l2.assume_init_ref();
            assert!(!l2r.is_null());
            for i in 0..block::ENTRY_COUNT * 3 + 1 {
                let l1i = i / block::ENTRY_COUNT;
                let l0i = i % block::ENTRY_COUNT;
                let l1r = l2r.as_ref_array()[l1i].assume_init_ref();
                assert!(!l1r.is_null());
                assert!(!l1r.as_ref_array()[l0i].assume_init_ref().is_null());
            }
        }
        assert_eq!(
            A_COUNTER.load(Ordering::Acquire),
            L0_BLOCKS + // L0
            L1_BLOCKS * block::ENTRY_COUNT + L1_BLOCKS + // L1
            block::ENTRY_COUNT * 3 + 1 + 4 + 1
        );
        bvec.resize(L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + block::ENTRY_COUNT * 2 + 1)
            .unwrap();
        unsafe {
            for i in 0..L0_BLOCKS {
                assert!(!bvec.l0[i].assume_init_ref().is_null());
            }
            for i in 0..L1_BLOCKS * block::ENTRY_COUNT {
                let l1i = i / block::ENTRY_COUNT;
                let l0i = i % block::ENTRY_COUNT;
                let l1r = bvec.l1[l1i].assume_init_ref();
                assert!(!l1r.is_null());
                assert!(!l1r.as_ref_array()[l0i].assume_init_ref().is_null());
            }
            let l2r = bvec.l2.assume_init_ref();
            assert!(!l2r.is_null());
            for i in 0..block::ENTRY_COUNT * 2 + 1 {
                let l1i = i / block::ENTRY_COUNT;
                let l0i = i % block::ENTRY_COUNT;
                let l1r = l2r.as_ref_array()[l1i].assume_init_ref();
                assert!(!l1r.is_null());
                assert!(!l1r.as_ref_array()[l0i].assume_init_ref().is_null());
            }
        }
        assert_eq!(
            A_COUNTER.load(Ordering::Acquire),
            L0_BLOCKS + // L0
            L1_BLOCKS * block::ENTRY_COUNT + L1_BLOCKS + // L1
            block::ENTRY_COUNT * 2 + 1 + 3 + 1
        );
        bvec.resize(L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + 1)
            .unwrap();
        unsafe {
            for i in 0..L0_BLOCKS {
                assert!(!bvec.l0[i].assume_init_ref().is_null());
            }
            for i in 0..L1_BLOCKS * block::ENTRY_COUNT {
                let l1i = i / block::ENTRY_COUNT;
                let l0i = i % block::ENTRY_COUNT;
                let l1r = bvec.l1[l1i].assume_init_ref();
                assert!(!l1r.is_null());
                assert!(!l1r.as_ref_array()[l0i].assume_init_ref().is_null());
            }
            let l2r = bvec.l2.assume_init_ref();
            assert!(!l2r.is_null());
            let l1r = l2r.as_ref_array()[0].assume_init_ref();
            assert!(!l1r.is_null());
            assert!(!l1r.as_ref_array()[0].assume_init_ref().is_null());
        }
        assert_eq!(
            A_COUNTER.load(Ordering::Acquire),
            L0_BLOCKS + // L0
            L1_BLOCKS * block::ENTRY_COUNT + L1_BLOCKS + // L1
            1 + 1 + 1
        );
        bvec.resize(L0_BLOCKS + 3 * block::ENTRY_COUNT + 1).unwrap();
        unsafe {
            for i in 0..L0_BLOCKS {
                assert!(!bvec.l0[i].assume_init_ref().is_null());
            }
            for i in 0..3 * block::ENTRY_COUNT + 1 {
                let l1i = i / block::ENTRY_COUNT;
                let l0i = i % block::ENTRY_COUNT;
                let l1r = bvec.l1[l1i].assume_init_ref();
                assert!(!l1r.is_null());
                assert!(!l1r.as_ref_array()[l0i].assume_init_ref().is_null());
            }
            let l2r = bvec.l2.assume_init_ref();
            assert!(l2r.is_null());
        }
        assert_eq!(
            A_COUNTER.load(Ordering::Acquire),
            L0_BLOCKS + // L0
            3 * block::ENTRY_COUNT + 1 + 4
        );
        bvec.resize(L0_BLOCKS).unwrap();
        unsafe {
            for i in 0..L0_BLOCKS {
                assert!(!bvec.l0[i].assume_init_ref().is_null());
            }
            assert!(bvec.l1[0].assume_init_ref().is_null());
        }
        assert_eq!(A_COUNTER.load(Ordering::Acquire), L0_BLOCKS);
        bvec.resize(12).unwrap();
        unsafe {
            for i in 0..12 {
                assert!(!bvec.l0[i].assume_init_ref().is_null());
            }
        }
        assert_eq!(A_COUNTER.load(Ordering::Acquire), 12);
        bvec.resize(0).unwrap();
        unsafe {
            for i in 0..L0_BLOCKS {
                assert!(bvec.l0[i].assume_init_ref().is_null());
            }
        }
        assert_eq!(A_COUNTER.load(Ordering::Acquire), 0);
    }
    #[test]
    fn bvec_index_l0() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(L0_BLOCKS).unwrap();
        for i in 0..L0_BLOCKS {
            let block = &bvec[i];
            assert_eq!(block as *const _, bvec.l0[i].as_ptr());
        }
    }
    #[test]
    fn bvec_index_l1() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(L0_BLOCKS + block::ENTRY_COUNT * 2 + 3).unwrap();
        for i in 0..block::ENTRY_COUNT * 2 + 3 {
            let l1i = i / block::ENTRY_COUNT;
            let l0i = i % block::ENTRY_COUNT;
            let block = &bvec[i + L0_BLOCKS];
            let l1r = unsafe { bvec.l1[l1i].assume_init_ref() };
            assert_eq!(block as *const _, l1r.as_ref_array()[l0i].as_ptr());
        }
    }
    #[test]
    fn bvec_index_l2() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + 3)
            .unwrap();
        for i in 0..3 {
            let l1i = i / block::ENTRY_COUNT;
            let l0i = i % block::ENTRY_COUNT;
            let block = &bvec[i + L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT];
            let l2r = unsafe { bvec.l2.assume_init_ref() };
            let l1r = unsafe { l2r.as_ref_array()[l1i].assume_init_ref() };
            assert_eq!(block as *const _, l1r.as_ref_array()[l0i].as_ptr());
        }
    }
    #[test]
    #[should_panic]
    fn bvec_index_invalid_l0_0() {
        let bvec = Bvec::new(TestAlloc {});
        let _block = &bvec[0];
    }
    #[test]
    #[should_panic]
    fn bvec_index_invalid_l0_1() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(13).unwrap();
        let _block = &bvec[15];
    }
    #[test]
    #[should_panic]
    fn bvec_index_invalid_l1_0() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(13).unwrap();
        let _block = &bvec[L0_BLOCKS + 2];
    }
    #[test]
    #[should_panic]
    fn bvec_index_invalid_l1_1() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(L0_BLOCKS + block::ENTRY_COUNT * 2 + 3).unwrap();
        let _block = &bvec[L0_BLOCKS + block::ENTRY_COUNT * 2 + 6];
    }
    #[test]
    #[should_panic]
    fn bvec_index_invalid_l1_2() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(L0_BLOCKS + block::ENTRY_COUNT * 2 + 3).unwrap();
        let _block = &bvec[L0_BLOCKS + block::ENTRY_COUNT * 3 + 1];
    }
    #[test]
    #[should_panic]
    fn bvec_index_invalid_l2_0() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(13).unwrap();
        let _block = &bvec[L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + 3];
    }
    #[test]
    #[should_panic]
    fn bvec_index_invalid_l2_1() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(L0_BLOCKS + block::ENTRY_COUNT * 3 + 13)
            .unwrap();
        let _block = &bvec[L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + 3];
    }
    #[test]
    #[should_panic]
    fn bvec_index_invalid_l2_2() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + 6)
            .unwrap();
        let _block = &bvec[L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + 8];
    }
    #[test]
    #[should_panic]
    fn bvec_index_invalid_l2_3() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + block::ENTRY_COUNT * 2 + 7)
            .unwrap();
        let _block =
            &bvec[L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + block::ENTRY_COUNT * 2 + 13];
    }
    #[test]
    #[should_panic]
    fn bvec_index_invalid_l2_4() {
        let mut bvec = Bvec::new(TestAlloc {});
        bvec.resize(L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + block::ENTRY_COUNT * 2 + 13)
            .unwrap();
        let _block = &bvec[L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + block::ENTRY_COUNT * 3 + 2];
    }
    #[test]
    fn bvec_write_read() {
        let mut bvec = Bvec::new(TestAlloc {});
        const N: usize = block::SIZE * (L0_BLOCKS + L1_BLOCKS * block::ENTRY_COUNT + 3);
        let mut data = vec![0u8; N];
        for i in 0..N {
            data[i] = (i & 0xFF) as u8;
        }
        assert_eq!(bvec.write(0, &data[..]), Ok(N));
        let mut buf = vec![0u8; 327];
        let mut off = 0usize;
        let mut rem = N;
        while rem != 0 {
            let count = min(rem, buf.len());
            assert_eq!(bvec.read(off, &mut buf[..]), Ok(count));
            for i in 0..count {
                assert_eq!(buf[i], ((i + off) & 0xFF) as u8);
            }
            rem -= count;
            off += count;
        }
    }
}
