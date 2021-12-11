use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::table::{Entry, MapAttributes, Space},
};
use core::ops::{Index, IndexMut};
use libsys::{error::Errno, mem::memset};

use super::RawAttributesImpl;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct EntryImpl(u64);

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
        todo!()
    }

    #[inline]
    fn block(addr: usize, attrs: MapAttributes) -> Self {
        todo!()
    }

    #[inline]
    fn address(self) -> usize {
        todo!()
    }

    #[inline]
    fn set_address(&mut self, virt: usize) {
        todo!()
    }

    #[inline]
    fn is_present(self) -> bool {
        todo!()
    }

    #[inline]
    fn is_normal(self) -> bool {
        todo!()
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
        todo!()
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
        todo!()
    }

    fn read_last_level(&self, virt: usize) -> Result<Self::Entry, Errno> {
        todo!()
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
        todo!()
    }

    /// Returns next-level translation table reference for `index`, if one is present.
    /// Same as [next_level_table_or_alloc], but returns `None` if no table is mapped.
    pub fn next_level_table(&self, index: usize) -> Option<&'static Self> {
        todo!()
    }

    /// Returns mutable next-level translation table reference for `index`,
    /// if one is present. Same as [next_level_table_or_alloc], but returns
    /// `None` if no table is mapped.
    pub fn next_level_table_mut(&mut self, index: usize) -> Option<&'static mut Self> {
        todo!()
    }
}

impl Index<usize> for TableImpl {
    type Output = EntryImpl;

    fn index(&self, index: usize) -> &Self::Output {
        todo!()
    }
}

impl IndexMut<usize> for TableImpl {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        todo!()
    }
}
