use crate::arch::intrin;
use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::table::{Entry, MapAttributes, Space},
};
use core::fmt;
use core::ops::{Index, IndexMut};
use libsys::error::Errno;

use super::{RawAttributesImpl, KERNEL_FIXED};

/// Transparent wrapper structure representing a single
/// translation table entry
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct EntryImpl(u64);

/// Structure describing a single level of translation mappings
#[derive(Clone, Copy)]
#[repr(C, align(0x1000))]
pub struct TableImpl {
    entries: [EntryImpl; 512],
}

/// Top-level translation table wrapper
#[repr(transparent)]
pub struct SpaceImpl(TableImpl);

impl EntryImpl {
    pub(super) const PRESENT: u64 = 1 << 0;
    pub(super) const WRITE: u64 = 1 << 1;
    pub(super) const USER: u64 = 1 << 2;
    pub(super) const BLOCK: u64 = 1 << 7;
    pub(super) const EX_COW: u64 = 1 << 62;

    const PHYS_MASK: u64 = 0x0000FFFFFFFFF000;
}

impl fmt::Debug for EntryImpl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("EntryImpl")
            .field("address", &self.address())
            .field("flags", &unsafe {
                RawAttributesImpl::from_bits_unchecked(self.0 & 0xFFF)
            })
            .finish_non_exhaustive()
    }
}

impl Entry for EntryImpl {
    type RawAttributes = RawAttributesImpl;
    const EMPTY: Self = Self(0);

    #[inline]
    fn normal(addr: usize, attrs: MapAttributes) -> Self {
        Self((addr as u64) | RawAttributesImpl::from(attrs).bits() | Self::PRESENT)
    }

    #[inline]
    fn block(addr: usize, attrs: MapAttributes) -> Self {
        Self((addr as u64) | RawAttributesImpl::from(attrs).bits() | Self::BLOCK | Self::PRESENT)
    }

    #[inline]
    fn address(self) -> usize {
        (self.0 & Self::PHYS_MASK) as usize
    }

    #[inline]
    fn set_address(&mut self, virt: usize) {
        self.0 &= !Self::PHYS_MASK;
        self.0 |= (virt as u64) & Self::PHYS_MASK;
    }

    #[inline]
    fn is_present(self) -> bool {
        self.0 & Self::PRESENT != 0
    }

    #[inline]
    fn is_normal(self) -> bool {
        self.0 & Self::BLOCK == 0
    }

    #[inline]
    fn fork_with_cow(&mut self) -> Self {
        self.0 &= !Self::WRITE;
        self.0 |= Self::EX_COW;
        *self
    }

    #[inline]
    fn copy_from_cow(self, new_addr: usize) -> Self {
        let attrs = self.0 & !(Self::PHYS_MASK | Self::EX_COW);
        Self(((new_addr as u64) & Self::PHYS_MASK) | (attrs | Self::WRITE))
    }

    #[inline]
    fn is_cow(self) -> bool {
        self.0 & Self::EX_COW != 0
    }

    #[inline]
    fn is_user_writable(self) -> bool {
        const BITS: u64 = EntryImpl::USER | EntryImpl::WRITE;
        self.0 & BITS == BITS
    }
}

impl Space for SpaceImpl {
    type Entry = EntryImpl;

    fn alloc_empty() -> Result<&'static mut Self, Errno> {
        let kernel_pdpt_phys =
            unsafe { &KERNEL_FIXED.pdpt as *const _ as usize - mem::KERNEL_OFFSET };
        let page = phys::alloc_page(PageUsage::Paging)?;
        let res = unsafe { &mut *(mem::virtualize(page) as *mut Self) };
        res.0.entries[..511].fill(EntryImpl::EMPTY);
        res.0.entries[511] = EntryImpl::normal(
            kernel_pdpt_phys,
            MapAttributes::SHARE_OUTER
                | MapAttributes::KERNEL_EXEC
                | MapAttributes::KERNEL_WRITE
                | MapAttributes::USER_READ
                | MapAttributes::USER_WRITE,
        );
        Ok(res)
    }

    unsafe fn release(space: &'static mut Self) {
        let pdpt0 = space.0.next_level_table_mut(0).unwrap();

        for pdpti in 0..512 {
            let pdpt_entry = pdpt0[pdpti];
            if !pdpt_entry.is_present() {
                continue;
            }

            assert!(pdpt_entry.is_normal());
            let pd = &mut *(mem::virtualize(pdpt_entry.address()) as *mut TableImpl);

            for pdi in 0..512 {
                let pd_entry = pd[pdi];
                if !pd_entry.is_present() {
                    continue;
                }

                assert!(pd_entry.is_normal());
                let pt = &mut *(mem::virtualize(pd_entry.address()) as *mut TableImpl);

                for pti in 0..512 {
                    let entry = pt[pti];

                    if !entry.is_present() {
                        continue;
                    }

                    assert!(entry.is_normal());
                    phys::free_page(entry.address()).unwrap();
                }
                phys::free_page(pd_entry.address()).unwrap();
            }
            phys::free_page(pdpt_entry.address()).unwrap();
        }
        phys::free_page(space.0[0].address()).unwrap();
    }

    fn fork(&mut self) -> Result<&'static mut Self, Errno> {
        let res = Self::alloc_empty()?;
        let pdpt0 = self.0.next_level_table_mut(0).unwrap();

        for pdpti in 0..512 {
            if let Some(pd) = pdpt0.next_level_table_mut(pdpti) {
                for pdi in 0..512 {
                    if let Some(pt) = pd.next_level_table_mut(pdi) {
                        for pti in 0..512 {
                            let entry = &mut pt[pti];
                            let virt_addr = (pdpti << 30) | (pdi << 21) | (pti << 12);

                            if !entry.is_present() {
                                continue;
                            }

                            assert!(entry.is_normal());
                            let src_phys = entry.address();

                            // let dst_phys = phys::alloc_page(PageUsage::UserPrivate)?;
                            // unsafe {
                            //     use libsys::mem::memcpy;
                            //     memcpy(
                            //         mem::virtualize(dst_phys) as *mut u8,
                            //         mem::virtualize(src_phys) as *const u8,
                            //         4096
                            //     );
                            // }

                            // debugln!("Clone page {:#x}", virt_addr);
                            // let new_entry = EntryImpl::normal(dst_phys, MapAttributes::USER_WRITE | MapAttributes::USER_READ);

                            // TODO check exact page usage
                            let dst_phys = unsafe { phys::fork_page(src_phys)? };

                            let new_entry = if dst_phys != src_phys {
                                todo!()
                            } else if entry.is_user_writable() {
                                entry.fork_with_cow()
                            } else {
                                *entry
                            };

                            unsafe {
                                res.write_last_level(virt_addr, new_entry, true, false)?;
                                intrin::flush_tlb_virt(virt_addr);
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
        let l0i = virt >> 39;
        let l1i = (virt >> 30) & 0x1FF;
        let l2i = (virt >> 21) & 0x1FF;
        let l3i = (virt >> 12) & 0x1FF;

        let l0_table = self.0.next_level_table_or_alloc(l0i)?;
        let l1_table = l0_table.next_level_table_or_alloc(l1i)?;
        let l2_table = l1_table.next_level_table_or_alloc(l2i)?;

        if l2_table[l3i].is_present() && !overwrite {
            warnln!(
                "Entry already exists for address: virt={:#x}, prev={:#x}, new={:#x}",
                virt,
                l2_table[l3i].address(),
                entry.address()
            );
            Err(Errno::AlreadyExists)
        } else {
            l2_table[l3i] = entry;
            intrin::flush_tlb_virt(virt);
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
            self[index] = EntryImpl::normal(
                phys,
                MapAttributes::USER_WRITE
                    | MapAttributes::USER_READ
                    | MapAttributes::KERNEL_WRITE
                    | MapAttributes::USER_EXEC,
            );
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
