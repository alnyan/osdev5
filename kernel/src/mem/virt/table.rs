//! Translation table manipulation facilities

use crate::mem::{
    self,
    phys::{self, PageUsage},
};
use core::ops::{Index, IndexMut};
use error::Errno;

/// Transparent wrapper structure representing a single
/// translation table entry
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Entry(u64);

/// Structure describing a single level of translation mappings
#[repr(C, align(0x1000))]
pub struct Table {
    entries: [Entry; 512],
}

/// Wrapper for top-most level of address translation tables
#[repr(transparent)]
pub struct Space(Table);

bitflags! {
    /// Attributes attached to each translation [Entry]
    pub struct MapAttributes: u64 {
        // TODO use 2 lower bits to determine mapping size?
        /// nG bit -- determines whether a TLB entry associated with this mapping
        ///           applies only to current ASID or all ASIDs.
        const NOT_GLOBAL = 1 << 11;
        /// AF bit -- must be set by software, otherwise Access Error exception is
        ///           generated when the page is accessed
        const ACCESS = 1 << 10;
        /// The memory region is outer-shareable
        const SH_OUTER = 2 << 8;
        /// This page is used for device-MMIO mapping and uses MAIR attribute #1
        const DEVICE = 1 << 2;

        /// UXN bit -- if set, page may not be used for instruction fetching from EL0
        const UXN = 1 << 54;
        /// PXN bit -- if set, page may not be used for instruction fetching from EL1
        const PXN = 1 << 53;

        // AP field
        // Default behavior is: read-write for EL1, no access for EL0
        /// If set, the page referred to by this entry is read-only for both EL0/EL1
        const AP_BOTH_READONLY = 3 << 6;
        /// If set, the page referred to by this entry is read-write for both EL0/EL1
        const AP_BOTH_READWRITE = 1 << 6;
    }
}

impl Table {
    /// Returns next-level translation table reference for `index`, if one is present.
    /// If `index` represents a `Block`-type mapping, will return an error.
    /// If `index` does not map to any translation table, will try to allocate, init and
    /// map a new one, returning it after doing so.
    pub fn next_level_table_or_alloc(&mut self, index: usize) -> Result<&'static mut Table, Errno> {
        let entry = self[index];
        if entry.is_present() {
            if !entry.is_table() {
                return Err(Errno::InvalidArgument);
            }

            Ok(unsafe { &mut *(mem::virtualize(entry.address_unchecked()) as *mut _) })
        } else {
            let phys = phys::alloc_page(PageUsage::Paging)?;
            let res = unsafe { &mut *(mem::virtualize(phys) as *mut Self) };
            self[index] = Entry::table(phys, MapAttributes::empty());
            res.entries.fill(Entry::invalid());
            Ok(res)
        }
    }

    pub fn next_level_table(&mut self, index: usize) -> Option<&'static mut Table> {
        let entry = self[index];
        if entry.is_present() {
            if !entry.is_table() {
                panic!("Entry is not a table: idx={}", index);
            }

            Some(unsafe { &mut *(mem::virtualize(entry.address_unchecked()) as *mut _) })
        } else {
            None
        }
    }

    /// Constructs and fills a [Table] with non-present mappings
    pub const fn empty() -> Table {
        Table {
            entries: [Entry::invalid(); 512],
        }
    }
}

impl Index<usize> for Table {
    type Output = Entry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for Table {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl Entry {
    const PRESENT: u64 = 1 << 0;
    const TABLE: u64 = 1 << 1;
    const PHYS_MASK: u64 = 0x0000FFFFFFFFF000;

    /// Constructs a single non-present mapping
    pub const fn invalid() -> Self {
        Self(0)
    }

    /// Constructs a `Block`-type memory mapping
    pub const fn block(phys: usize, attrs: MapAttributes) -> Self {
        Self((phys as u64 & Self::PHYS_MASK) | attrs.bits() | Self::PRESENT)
    }

    /// Constructs a `Table` or `Page`-type mapping depending on translation level
    /// this entry is used at
    pub const fn table(phys: usize, attrs: MapAttributes) -> Self {
        Self((phys as u64 & Self::PHYS_MASK) | attrs.bits() | Self::PRESENT | Self::TABLE)
    }

    /// Returns `true` if this entry is not invalid
    pub const fn is_present(self) -> bool {
        self.0 & Self::PRESENT != 0
    }

    /// Returns `true` if this entry is a `Table` or `Page`-type mapping
    pub const fn is_table(self) -> bool {
        self.0 & Self::TABLE != 0
    }

    /// Returns the target address of this translation entry.
    ///
    /// # Safety
    ///
    /// Does not check if the entry is actually valid.
    pub const unsafe fn address_unchecked(self) -> usize {
        (self.0 & Self::PHYS_MASK) as usize
    }

    unsafe fn fork_flags(self) -> MapAttributes {
        MapAttributes::from_bits_unchecked(self.0 & !Self::PHYS_MASK)
    }
}

impl Space {
    /// Creates a new virtual address space and fills it with [Entry::invalid()]
    /// mappings. Does physical memory page allocation.
    pub fn alloc_empty() -> Result<&'static mut Self, Errno> {
        let phys = phys::alloc_page(PageUsage::Paging)?;
        let res = unsafe { &mut *(mem::virtualize(phys) as *mut Self) };
        res.0.entries.fill(Entry::invalid());
        Ok(res)
    }

    /// Inserts a single `virt` -> `phys` translation entry to this address space.
    ///
    /// TODO: only works with 4K-sized pages at this moment.
    pub fn map(&mut self, virt: usize, phys: usize, flags: MapAttributes) -> Result<(), Errno> {
        let l0i = virt >> 30;
        let l1i = (virt >> 21) & 0x1FF;
        let l2i = (virt >> 12) & 0x1FF;

        let l1_table = self.0.next_level_table_or_alloc(l0i)?;
        let l2_table = l1_table.next_level_table_or_alloc(l1i)?;

        if l2_table[l2i].is_present() {
            Err(Errno::AlreadyExists)
        } else {
            l2_table[l2i] = Entry::table(phys, flags | MapAttributes::ACCESS);
            debugln!("Map {:#x} -> {:#x}", virt, phys);
            Ok(())
        }
    }

    pub fn fork(&mut self) -> Result<&'static mut Self, Errno> {
        let mut res = Self::alloc_empty()?;
        for l0i in 0..512 {
            if let Some(l1_table) = self.0.next_level_table(l0i) {
                for l1i in 0..512 {
                    if let Some(l2_table) = l1_table.next_level_table(l1i) {
                        for l2i in 0..512 {
                            let entry = l2_table[l2i];

                            // TODO copy-on-write
                            if entry.is_present() {
                                assert!(entry.is_table());
                                let src_phys = unsafe { entry.address_unchecked() };
                                let virt_addr = (l0i << 30) | (l1i << 21) | (l2i << 12);
                                debugln!("Fork page {:#x}:{:#x}", virt_addr, src_phys);
                                let dst_phys = phys::clone_page(src_phys)?;

                                res.map(virt_addr, dst_phys, unsafe { entry.fork_flags() })?;
                            }
                        }
                    }
                }
            }
        }
        Ok(res)
    }
}
