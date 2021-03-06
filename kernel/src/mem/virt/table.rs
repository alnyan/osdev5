//! Translation table manipulation facilities

use crate::mem::{
    self,
    phys::{self, PageUsage},
};
use core::ops::{Index, IndexMut};
use libsys::{error::Errno, mem::memset};

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

        /// Pages marked with this bit are Copy-on-Write
        const EX_COW = 1 << 55;

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

    /// Returns next-level translation table reference for `index`, if one is present.
    /// Same as [next_level_table_or_alloc], but returns `None` if no table is mapped.
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

    unsafe fn set_address(&mut self, address: usize) {
        self.0 &= !Self::PHYS_MASK;
        self.0 |= (address as u64) & Self::PHYS_MASK;
    }

    unsafe fn fork_flags(self) -> MapAttributes {
        MapAttributes::from_bits_unchecked(self.0 & !Self::PHYS_MASK)
    }

    fn set_cow(&mut self) {
        self.0 |= (MapAttributes::AP_BOTH_READONLY | MapAttributes::EX_COW).bits();
    }

    fn clear_cow(&mut self) {
        self.0 &= !(MapAttributes::AP_BOTH_READONLY | MapAttributes::EX_COW).bits();
        self.0 |= MapAttributes::AP_BOTH_READWRITE.bits();
    }

    #[inline]
    fn is_cow(self) -> bool {
        let attrs = (MapAttributes::AP_BOTH_READONLY | MapAttributes::EX_COW).bits();
        self.0 & attrs == attrs
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
            #[cfg(feature = "verbose")]
            debugln!("{:#p} Map {:#x} -> {:#x}, {:?}", self, virt, phys, flags);
            Ok(())
        }
    }

    /// Translates a virtual address into a corresponding physical one.
    ///
    /// Only works for 4K pages atm.
    // TODO extract attributes
    pub fn translate(&mut self, virt: usize) -> Result<usize, Errno> {
        let l0i = virt >> 30;
        let l1i = (virt >> 21) & 0x1FF;
        let l2i = (virt >> 12) & 0x1FF;

        let l1_table = self.0.next_level_table(l0i).ok_or(Errno::DoesNotExist)?;
        let l2_table = l1_table.next_level_table(l1i).ok_or(Errno::DoesNotExist)?;

        let entry = l2_table[l2i];
        if entry.is_present() {
            Ok(unsafe { entry.address_unchecked() })
        } else {
            Err(Errno::DoesNotExist)
        }
    }

    /// Attempts to resolve a page fault at `virt` address by copying the
    /// underlying Copy-on-Write mapping (if any is present)
    pub fn try_cow_copy(&mut self, virt: usize) -> Result<(), Errno> {
        let virt = virt & !0xFFF;
        let l0i = virt >> 30;
        let l1i = (virt >> 21) & 0x1FF;
        let l2i = (virt >> 12) & 0x1FF;

        let l1_table = self.0.next_level_table(l0i).ok_or(Errno::DoesNotExist)?;
        let l2_table = l1_table.next_level_table(l1i).ok_or(Errno::DoesNotExist)?;

        let entry = l2_table[l2i];

        if !entry.is_present() {
            warnln!("Entry is not present: {:#x}", virt);
            return Err(Errno::DoesNotExist);
        }

        let src_phys = unsafe { entry.address_unchecked() };
        if !entry.is_cow() {
            warnln!(
                "Entry is not marked as CoW: {:#x}, points to {:#x}",
                virt,
                src_phys
            );
            return Err(Errno::DoesNotExist);
        }

        let dst_phys = unsafe { phys::copy_cow_page(src_phys)? };
        unsafe {
            l2_table[l2i].set_address(dst_phys);
        }
        l2_table[l2i].clear_cow();

        Ok(())
    }

    /// Allocates a contiguous region from the address space and maps
    /// physical pages to it
    pub fn allocate(
        &mut self,
        start: usize,
        end: usize,
        len: usize,
        flags: MapAttributes,
        usage: PageUsage,
    ) -> Result<usize, Errno> {
        'l0: for page in (start..end).step_by(0x1000) {
            for i in 0..len {
                if self.translate(page + i * 0x1000).is_ok() {
                    continue 'l0;
                }
            }

            for i in 0..len {
                let phys = phys::alloc_page(usage).unwrap();
                self.map(page + i * 0x1000, phys, flags).unwrap();
            }
            return Ok(page);
        }
        Err(Errno::OutOfMemory)
    }

    /// Removes a single 4K page mapping from the table and
    /// releases the underlying physical memory
    pub fn unmap_single(&mut self, page: usize) -> Result<(), Errno> {
        let l0i = page >> 30;
        let l1i = (page >> 21) & 0x1FF;
        let l2i = (page >> 12) & 0x1FF;

        let l1_table = self.0.next_level_table(l0i).ok_or(Errno::DoesNotExist)?;
        let l2_table = l1_table.next_level_table(l1i).ok_or(Errno::DoesNotExist)?;

        let entry = l2_table[l2i];

        if !entry.is_present() {
            return Err(Errno::DoesNotExist);
        }

        let phys = unsafe { entry.address_unchecked() };
        unsafe {
            phys::free_page(phys)?;
        }
        l2_table[l2i] = Entry::invalid();

        unsafe {
            asm!("tlbi vaae1, {}", in(reg) page);
        }

        // TODO release paging structure memory

        Ok(())
    }

    /// Releases a range of virtual pages and their corresponding physical pages
    pub fn free(&mut self, start: usize, len: usize) -> Result<(), Errno> {
        for i in 0..len {
            self.unmap_single(start + i * 0x1000)?;
        }
        Ok(())
    }

    /// Performs a copy of the address space, cloning data owned by it
    pub fn fork(&mut self) -> Result<&'static mut Self, Errno> {
        let res = Self::alloc_empty()?;
        for l0i in 0..512 {
            if let Some(l1_table) = self.0.next_level_table(l0i) {
                for l1i in 0..512 {
                    if let Some(l2_table) = l1_table.next_level_table(l1i) {
                        for l2i in 0..512 {
                            let entry = l2_table[l2i];

                            if !entry.is_present() {
                                continue;
                            }

                            assert!(entry.is_table());
                            let src_phys = unsafe { entry.address_unchecked() };
                            let virt_addr = (l0i << 30) | (l1i << 21) | (l2i << 12);
                            let dst_phys = unsafe { phys::fork_page(src_phys)? };

                            let mut flags = unsafe { entry.fork_flags() };
                            if dst_phys != src_phys {
                                todo!();
                                // res.map(virt_addr, dst_phys, flags)?;
                            } else {
                                let writable = flags & MapAttributes::AP_BOTH_READONLY
                                    == MapAttributes::AP_BOTH_READWRITE;

                                if writable {
                                    flags |=
                                        MapAttributes::AP_BOTH_READONLY | MapAttributes::EX_COW;
                                    l2_table[l2i].set_cow();

                                    unsafe {
                                        asm!("tlbi vaae1, {}", in(reg) virt_addr);
                                    }
                                }

                                res.map(virt_addr, dst_phys, flags)?;
                            }
                        }
                    }
                }
            }
        }
        Ok(res)
    }

    /// Releases all the mappings from the address space. Frees all
    /// memory pages referenced by this space as well as those used for
    /// its paging tables.
    ///
    /// # Safety
    ///
    /// Unsafe: may invalidate currently active address space
    pub unsafe fn release(space: &mut Self) {
        for l0i in 0..512 {
            let l0_entry = space.0[l0i];
            if !l0_entry.is_present() {
                continue;
            }

            assert!(l0_entry.is_table());
            let l1_table = &mut *(mem::virtualize(l0_entry.address_unchecked()) as *mut Table);

            for l1i in 0..512 {
                let l1_entry = l1_table[l1i];
                if !l1_entry.is_present() {
                    continue;
                }
                assert!(l1_entry.is_table());
                let l2_table = &mut *(mem::virtualize(l1_entry.address_unchecked()) as *mut Table);

                for l2i in 0..512 {
                    let entry = l2_table[l2i];
                    if !entry.is_present() {
                        continue;
                    }

                    assert!(entry.is_table());
                    phys::free_page(entry.address_unchecked()).unwrap();
                }
                phys::free_page(l1_entry.address_unchecked()).unwrap();
            }
            phys::free_page(l0_entry.address_unchecked()).unwrap();
        }
        memset(space as *mut Space as *mut u8, 0, 4096);
    }

    /// Returns the physical address of this structure
    pub fn address_phys(&mut self) -> usize {
        (self as *mut _ as usize) - mem::KERNEL_OFFSET
    }
}
