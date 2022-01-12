use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::table::{Entry, MapAttributes, Space},
};
use core::ops::{Index, IndexMut};
use libsys::{error::Errno, mem::memset};

use super::{RawAttributesImpl, KERNEL_FIXED};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct EntryImpl(u64);

#[derive(Clone, Copy)]
#[repr(C, align(0x1000))]
pub struct TableImpl {
    entries: [EntryImpl; 512],
}

#[repr(transparent)]
pub struct SpaceImpl(TableImpl);

impl EntryImpl {}

impl Entry for EntryImpl {
    type RawAttributes = RawAttributesImpl;
    const EMPTY: Self = Self(0);

    #[inline]
    fn normal(addr: usize, attrs: MapAttributes) -> Self {
        Self((addr as u64) | attrs.bits() | (1 << 0))
    }

    #[inline]
    fn block(addr: usize, attrs: MapAttributes) -> Self {
        Self((addr as u64) | attrs.bits() | (1 << 7) | (1 << 0))
    }

    #[inline]
    fn address(self) -> usize {
        (self.0 & !0xFFF) as usize
    }

    #[inline]
    fn set_address(&mut self, virt: usize) {
        todo!()
    }

    #[inline]
    fn is_present(self) -> bool {
        self.0 & (1 << 0) != 0
    }

    #[inline]
    fn is_normal(self) -> bool {
        self.0 & (1 << 7) == 0
    }

    #[inline]
    fn fork_with_cow(&mut self) -> Self {
        todo!()
    }

    #[inline]
    fn copy_from_cow(self, new_addr: usize) -> Self {
        todo!()
    }

    #[inline]
    fn is_cow(self) -> bool {
        todo!()
    }

    #[inline]
    fn is_user_writable(self) -> bool {
        todo!()
    }
}

impl Space for SpaceImpl {
    type Entry = EntryImpl;

    fn alloc_empty() -> Result<&'static mut Self, Errno> {
        let kernel_pdpt_phys = unsafe {
            &KERNEL_FIXED.pdpt as *const _ as usize - mem::KERNEL_OFFSET
        };
        let page = phys::alloc_page(PageUsage::Paging)?;
        let res = unsafe { &mut *(mem::virtualize(page) as *mut Self) };
        res.0.entries[..511].fill(EntryImpl::EMPTY);
        res.0.entries[511] = EntryImpl::normal(kernel_pdpt_phys, MapAttributes::SHARE_OUTER | MapAttributes::KERNEL_EXEC | MapAttributes::KERNEL_WRITE);
        Ok(res)
    }

    unsafe fn release(space: &'static mut Self) {
        todo!()
    }

    fn fork(&mut self) -> Result<&'static mut Self, Errno> {
        todo!()
    }

    unsafe fn write_last_level(
        &mut self,
        virt: usize,
        entry: Self::Entry,
        _create_intermediate: bool, // TODO handle this properly
        overwrite: bool,
    ) -> Result<(), Errno> {
        let l0i = virt >> 39;
        let l1i = (virt >> 30) & 0x1FF;
        let l2i = (virt >> 21) & 0x1FF;
        let l3i = (virt >> 12) & 0x1FF;

        let l0_table = self.0.next_level_table_or_alloc(l0i)?;
        let l1_table = l0_table.next_level_table_or_alloc(l1i)?;
        let l2_table = l1_table.next_level_table_or_alloc(l2i)?;

        if l2_table[l3i].is_present() {
            warnln!("Entry already exists for address: virt={:#x}, prev={:#x}, new={:#x}", virt, l2_table[l3i].address(), entry.address());
            Err(Errno::AlreadyExists)
        } else {
            l2_table[l3i] = entry;
            unsafe {
                core::arch::asm!("invlpg ({})", in(reg) virt, options(att_syntax));
            }
            Ok(())
        }
    }

    fn read_last_level(&self, virt: usize) -> Result<Self::Entry, Errno> {
        let l0i = virt >> 39;
        let l1i = (virt >> 30) & 0x1FF;
        let l2i = (virt >> 21) & 0x1FF;
        let l3i = (virt >> 12) & 0x1FF;

        let l0_table = self.0.next_level_table(l0i).ok_or(Errno::DoesNotExist)?;
        let l1_table = l0_table.next_level_table(l1i).ok_or(Errno::DoesNotExist)?;
        let l2_table = l1_table.next_level_table(l2i).ok_or(Errno::DoesNotExist)?;

        let entry = l2_table[l3i];
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
            entries: [EntryImpl::EMPTY; 512]
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
            self[index] = EntryImpl::normal(phys, MapAttributes::USER_WRITE | MapAttributes::USER_READ);
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
