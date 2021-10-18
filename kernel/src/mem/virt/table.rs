use crate::mem::{
    self,
    phys::{self, PageUsage},
};
use core::ops::{Index, IndexMut};
use error::Errno;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Entry(u64);

#[repr(C, align(0x1000))]
pub struct Table {
    entries: [Entry; 512],
}

#[repr(transparent)]
pub struct Space(Table);

bitflags! {
    pub struct MapAttributes: u64 {
        // TODO use 2 lower bits to determine mapping size?
        const NOT_GLOBAL = 1 << 11;
        const ACCESS = 1 << 10;
        const SH_OUTER = 2 << 8;
        const DEVICE = 1 << 2;

        const UXN = 1 << 54;
        const PXN = 1 << 53;
    }
}

impl Table {
    pub fn next_level_table_or_alloc(&mut self, index: usize) -> Result<&'static mut Table, Errno> {
        let entry = self[index];
        if entry.is_present() {
            if !entry.is_table() {
                return Err(Errno::InvalidArgument);
            }

            Ok(unsafe { &mut *(mem::virtualize(entry.address_unchecked()) as *mut _) })
        } else {
            let phys = phys::alloc_page(PageUsage::Paging)?;
            debugln!("Allocated new page table at {:#x}", phys);
            let res = unsafe { &mut *(mem::virtualize(phys) as *mut Self) };
            self[index] = Entry::table(phys, MapAttributes::empty());
            res.entries.fill(Entry::invalid());
            Ok(res)
        }
    }

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

    pub const fn invalid() -> Self {
        Self(0)
    }

    pub const fn block(phys: usize, attrs: MapAttributes) -> Self {
        Self((phys as u64 & Self::PHYS_MASK) | attrs.bits() | Self::PRESENT)
    }

    pub const fn table(phys: usize, attrs: MapAttributes) -> Self {
        Self((phys as u64 & Self::PHYS_MASK) | attrs.bits() | Self::PRESENT | Self::TABLE)
    }

    pub const fn is_present(self) -> bool {
        self.0 & Self::PRESENT != 0
    }

    pub const fn is_table(self) -> bool {
        self.0 & Self::TABLE != 0
    }

    pub const unsafe fn address_unchecked(self) -> usize {
        (self.0 & Self::PHYS_MASK) as usize
    }
}

impl Space {
    pub fn alloc_empty() -> Result<&'static mut Self, Errno> {
        let phys = phys::alloc_page(PageUsage::Paging)?;
        let res = unsafe { &mut *(mem::virtualize(phys) as *mut Self) };
        res.0.entries.fill(Entry::invalid());
        Ok(res)
    }

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
}
