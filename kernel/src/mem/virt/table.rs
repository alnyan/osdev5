//! Translation table manipulation facilities

use crate::arch::platform::virt as virt_impl;
use crate::mem::{
    self,
    phys::{self, PageUsage},
};
use core::ffi::c_void;
use libsys::error::Errno;
pub use virt_impl::{EntryImpl, SpaceImpl};

bitflags! {
    /// Virtual space entry attributes
    pub struct MapAttributes: u64 {
        /// Entry is readable by user threads
        const USER_READ = 1 << 0;
        /// Entry is writable by user threads
        const USER_WRITE = 1 << 1;
        /// Data from entry can be executed by user threads
        const USER_EXEC = 1 << 2;

        /// Entry is writable by kernel
        const KERNEL_WRITE = 1 << 3;
        /// Data from entry can be executed by kernel
        const KERNEL_EXEC = 1 << 4;

        /// TODO TBD
        const SHARE_OUTER = 1 << 5;
        /// Memory is used for device interaction
        const DEVICE_MEMORY = 1 << 6;

        /// Entry is marked as Copy-on-Write
        const COPY_ON_WRITE = 1 << 7;

        /// Access flag for entry
        const ACCESS = 1 << 8;
        /// Entry is global across virtual address spaces
        const GLOBAL = 1 << 9;
    }
}

/// Interface for a single element of paging mapping
pub trait Entry: Clone + Copy {
    /// Platform-specific entry attribute representation
    type RawAttributes: From<MapAttributes> + Copy + Clone;
    /// Invalid entry with no association
    const EMPTY: Self;

    /// Constructs an entry pointing to next-level table or page
    fn normal(addr: usize, attrs: MapAttributes) -> Self;
    /// Constructs an entry pointing to a contiguous block
    fn block(addr: usize, attrs: MapAttributes) -> Self;

    /// Returns physical address the entry points to
    fn address(self) -> usize;
    /// Changes the entry physical address
    fn set_address(&mut self, value: usize);

    /// Marks page as CoW and removes user write ability
    fn fork_with_cow(&mut self) -> Self;
    /// Clones a CoW entry
    fn copy_from_cow(self, new_addr: usize) -> Self;

    /// Returns `true` if entry maps a paging element
    fn is_present(self) -> bool;
    /// Returns `true` if page is a 4KiB one
    fn is_normal(self) -> bool;
    /// Returns `true` if page is marked as Copy-on-Write
    fn is_cow(self) -> bool;
    /// Returns `true` if page is write-accessible for user threads
    fn is_user_writable(self) -> bool;
}

/// Interface for virtual address space manipulation
pub trait Space {
    /// Single table entry data type
    type Entry: Entry;

    /// Creates an empty address space
    fn alloc_empty() -> Result<&'static mut Self, Errno>;

    /// Removes all non-kernel entries from the space.
    ///
    /// # Safety
    ///
    /// Only safe to call on spaces not currently in use, otherwise will
    /// trigger undefined behavior and/or page fault.
    unsafe fn release(space: &'static mut Self);

    /// Forks a process virtual memory space
    fn fork(&mut self) -> Result<&'static mut Self, Errno>;

    /// Writes an entry corresponding to `virt` address
    /// to last-level table of this address space.
    ///
    /// # Safety
    ///
    /// Unsafe: arbitrary memory space manipulation.
    unsafe fn write_last_level(
        &mut self,
        virt: usize,
        entry: Self::Entry,
        create_intermediate: bool,
        overwrite: bool,
    ) -> Result<(), Errno>;

    /// Reads an entry corresponding to `virt` address
    fn read_last_level(&self, virt: usize) -> Result<Self::Entry, Errno>;

    /// Returns physical address of this table
    fn address_phys(&mut self) -> usize {
        self as *mut _ as *mut c_void as usize - mem::KERNEL_OFFSET
    }

    /// Performs Copy-on-Write cloning on page fault
    fn try_cow_copy(&mut self, virt: usize) -> Result<(), Errno> {
        let entry = self.read_last_level(virt)?;
        let src_phys = entry.address();

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
            self.write_last_level(virt, entry.copy_from_cow(dst_phys), false, true)?;
        }
        Ok(())
    }

    /// Creates a new virtual -> physical memory mapping. Will fail if one is
    /// already associated with given virtual address.
    fn map(&mut self, virt: usize, phys: usize, attrs: MapAttributes) -> Result<(), Errno> {
        debugln!("Map {:#x} -> {:#x}, {:?}", virt, phys, attrs);
        unsafe {
            self.write_last_level(
                virt,
                Entry::normal(phys, attrs | MapAttributes::ACCESS),
                true,
                false,
            )
        }
    }

    /// Returns a virtual address physical mapping destination
    fn translate(&mut self, virt: usize) -> Result<usize, Errno> {
        self.read_last_level(virt).map(Entry::address)
    }

    /// Releases memory from virtual address range `start`..`start + len * 0x1000`
    fn free(&mut self, start: usize, len: usize) -> Result<(), Errno> {
        for i in 0..len {
            unsafe {
                self.write_last_level(start + i * 0x1000, Self::Entry::EMPTY, false, true)?;
            }
        }
        Ok(())
    }

    /// Allocates a contiguous region from the address space and maps
    /// physical pages to it
    fn allocate(
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
}
