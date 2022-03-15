use crate::arch::aarch64::intrin::flush_tlb_virt;
use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::table::{Entry, MapAttributes, Space},
};
use core::ops::{Index, IndexMut};
use libsys::{error::Errno, mem::memset};

use super::RawAttributesImpl;

/// Transparent wrapper structure representing a single
/// translation table entry
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct EntryImpl(u64);

/// Structure describing a single level of translation mappings
#[repr(C, align(0x1000))]
pub struct TableImpl {
    entries: [EntryImpl; 512],
}

/// Top-level translation table wrapper
#[repr(transparent)]
pub struct SpaceImpl(TableImpl);

impl EntryImpl {
    const PRESENT: u64 = 1 << 0;
    const TABLE: u64 = 1 << 1;
    const PHYS_MASK: u64 = 0x0000FFFFFFFFF000;
}

impl Entry for EntryImpl {
    type RawAttributes = RawAttributesImpl;
    const EMPTY: Self = Self(0);

    #[inline]
    fn normal(addr: usize, attrs: MapAttributes) -> Self {
        Self((addr as u64) | RawAttributesImpl::from(attrs).bits() | (1 << 1) | (1 << 0))
    }

    #[inline]
    fn block(addr: usize, attrs: MapAttributes) -> Self {
        Self((addr as u64) | RawAttributesImpl::from(attrs).bits() | (1 << 0))
    }

    #[inline]
    fn address(self) -> usize {
        (self.0 & Self::PHYS_MASK) as usize
    }

    #[inline]
    fn set_address(&mut self, virt: usize) {
        self.0 = (self.0 & !Self::PHYS_MASK) | ((virt as u64) & Self::PHYS_MASK);
    }

    #[inline]
    fn is_present(self) -> bool {
        self.0 & Self::PRESENT != 0
    }

    #[inline]
    fn is_normal(self) -> bool {
        self.0 & Self::TABLE != 0
    }

    #[inline]
    fn fork_with_cow(&mut self) -> Self {
        self.0 |= (RawAttributesImpl::AP_BOTH_READONLY | RawAttributesImpl::EX_COW).bits();
        *self
    }

    #[inline]
    fn copy_from_cow(self, new_addr: usize) -> Self {
        let attrs = self.0
            & !(Self::PHYS_MASK
                | RawAttributesImpl::AP_BOTH_READONLY.bits()
                | RawAttributesImpl::EX_COW.bits());
        Self(
            ((new_addr as u64) & Self::PHYS_MASK)
                | (attrs | RawAttributesImpl::AP_BOTH_READWRITE.bits()),
        )
    }

    #[inline]
    fn is_cow(self) -> bool {
        self.0 & RawAttributesImpl::EX_COW.bits() != 0
    }

    #[inline]
    fn is_user_writable(self) -> bool {
        self.0 & RawAttributesImpl::AP_BOTH_READONLY.bits()
            == RawAttributesImpl::AP_BOTH_READWRITE.bits()
    }
}

impl Space for SpaceImpl {
    type Entry = EntryImpl;

    fn alloc_empty() -> Result<&'static mut Self, Errno> {
        let phys = phys::alloc_page(PageUsage::Paging)?;
        let res = unsafe { &mut *(mem::virtualize(phys) as *mut Self) };
        res.0.entries.fill(EntryImpl::EMPTY);
        Ok(res)
    }

    unsafe fn release(space: &'static mut Self) {
        for l0i in 0..512 {
            let l0_entry = space.0[l0i];
            if !l0_entry.is_present() {
                continue;
            }

            assert!(l0_entry.is_normal());
            let l1_table = &mut *(mem::virtualize(l0_entry.address()) as *mut TableImpl);

            for l1i in 0..512 {
                let l1_entry = l1_table[l1i];
                if !l1_entry.is_present() {
                    continue;
                }
                assert!(l1_entry.is_normal());
                let l2_table = &mut *(mem::virtualize(l1_entry.address()) as *mut TableImpl);

                for l2i in 0..512 {
                    let entry = l2_table[l2i];
                    if !entry.is_present() {
                        continue;
                    }

                    assert!(entry.is_normal());
                    phys::free_page(entry.address()).unwrap();
                }
                phys::free_page(l1_entry.address()).unwrap();
            }
            phys::free_page(l0_entry.address()).unwrap();
        }
        memset(space as *mut Self as *mut u8, 0, 4096);
    }

    fn fork(&mut self) -> Result<&'static mut Self, Errno> {
        let res = Self::alloc_empty()?;
        for l0i in 0..512 {
            if let Some(l1_table) = self.0.next_level_table_mut(l0i) {
                for l1i in 0..512 {
                    if let Some(l2_table) = l1_table.next_level_table_mut(l1i) {
                        for l2i in 0..512 {
                            let entry = &mut l2_table[l2i];
                            if !entry.is_present() {
                                continue;
                            }

                            assert!(entry.is_normal());
                            let src_phys = entry.address();
                            let virt_addr = (l0i << 30) | (l1i << 21) | (l2i << 12);
                            let dst_phys = unsafe { phys::fork_page(src_phys)? };

                            let new_entry = if dst_phys != src_phys {
                                todo!()
                            } else if entry.is_user_writable() {
                                entry.fork_with_cow()
                            } else {
                                *entry
                            };

                            unsafe {
                                flush_tlb_virt(virt_addr);
                                res.write_last_level(virt_addr, new_entry, true, false)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(res)
    }

    unsafe fn write_last_level(
        &mut self,
        virt: usize,
        entry: Self::Entry,
        _create_intermediate: bool, // TODO handle this properly
        overwrite: bool,
    ) -> Result<(), Errno> {
        let l0i = virt >> 30;
        let l1i = (virt >> 21) & 0x1FF;
        let l2i = (virt >> 12) & 0x1FF;

        let l1_table = self.0.next_level_table_or_alloc(l0i)?;
        let l2_table = l1_table.next_level_table_or_alloc(l1i)?;

        if l2_table[l2i].is_present() && !overwrite {
            return Err(Errno::AlreadyExists);
        };

        l2_table[l2i] = entry;
        #[cfg(feature = "verbose")]
        debugln!(
            "{:#p} Map {:#x} -> {:#x}, {:#x}",
            self,
            virt,
            entry.address(),
            entry.0 & !EntryImpl::PHYS_MASK
        );
        flush_tlb_virt(virt);
        Ok(())
    }

    fn read_last_level(&self, virt: usize) -> Result<Self::Entry, Errno> {
        let l0i = virt >> 30;
        let l1i = (virt >> 21) & 0x1FF;
        let l2i = (virt >> 12) & 0x1FF;

        let l1_table = self.0.next_level_table(l0i).ok_or(Errno::DoesNotExist)?;
        let l2_table = l1_table.next_level_table(l1i).ok_or(Errno::DoesNotExist)?;

        let entry = l2_table[l2i];
        if entry.is_present() {
            Ok(entry)
        } else {
            Err(Errno::DoesNotExist)
        }
    }
}

impl TableImpl {
    /// Constructs a table with no valid mappings
    pub const fn empty() -> Self {
        Self {
            entries: [EntryImpl::EMPTY; 512],
        }
    }

    /// Returns next-level translation table reference for `index`, if one is present.
    /// If `index` represents a `Block`-type mapping, will return an error.
    /// If `index` does not map to any translation table, will try to allocate, init and
    /// map a new one, returning it after doing so.
    pub fn next_level_table_or_alloc(&mut self, index: usize) -> Result<&'static mut Self, Errno> {
        let entry = self[index];
        if entry.is_present() {
            if !entry.is_normal() {
                return Err(Errno::InvalidArgument);
            }

            Ok(unsafe { &mut *(mem::virtualize(entry.address()) as *mut _) })
        } else {
            let phys = phys::alloc_page(PageUsage::Paging)?;
            let res = unsafe { &mut *(mem::virtualize(phys) as *mut Self) };
            self[index] = EntryImpl::normal(phys, MapAttributes::empty());
            res.entries.fill(EntryImpl::EMPTY);
            Ok(res)
        }
    }

    /// Returns next-level translation table reference for `index`, if one is present.
    /// Same as [next_level_table_or_alloc], but returns `None` if no table is mapped.
    pub fn next_level_table(&self, index: usize) -> Option<&'static Self> {
        let entry = self[index];
        if entry.is_present() {
            if !entry.is_normal() {
                panic!("Entry is not a table: idx={}", index);
            }

            Some(unsafe { &*(mem::virtualize(entry.address()) as *const _) })
        } else {
            None
        }
    }

    /// Returns mutable next-level translation table reference for `index`,
    /// if one is present. Same as [next_level_table_or_alloc], but returns
    /// `None` if no table is mapped.
    pub fn next_level_table_mut(&mut self, index: usize) -> Option<&'static mut Self> {
        let entry = self[index];
        if entry.is_present() {
            if !entry.is_normal() {
                panic!("Entry is not a table: idx={}", index);
            }

            Some(unsafe { &mut *(mem::virtualize(entry.address()) as *mut _) })
        } else {
            None
        }
    }
}

impl Index<usize> for TableImpl {
    type Output = EntryImpl;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for TableImpl {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}
