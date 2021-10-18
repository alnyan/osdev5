//!

use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::{MapAttributes, Space},
};
use error::Errno;

trait Elf {
    type Addr;
    type Half;
    type SHalf;
    type Off;
    type Sword;
    type Word;
    type Xword;
    type Sxword;
}

struct Elf64;

impl Elf for Elf64 {
    type Addr = u64;
    type Half = u16;
    type SHalf = i16;
    type Off = u64;
    type Sword = i32;
    type Word = u32;
    type Xword = u64;
    type Sxword = i64;
}

#[repr(C)]
struct Ehdr<E: Elf> {
    ident: [u8; 16],
    typ: E::Half,
    machine: E::Half,
    version: E::Word,
    entry: E::Addr,
    phoff: E::Off,
    shoff: E::Off,
    flags: E::Word,
    ehsize: E::Half,
    phentsize: E::Half,
    phnum: E::Half,
    shentsize: E::Half,
    shnum: E::Half,
    shstrndx: E::Half,
}

#[repr(C)]
struct Phdr<E: Elf> {
    typ: E::Word,
    flags: E::Word,
    offset: E::Off,
    vaddr: E::Addr,
    paddr: E::Addr,
    filesz: E::Xword,
    memsz: E::Xword,
    align: E::Xword,
}

unsafe fn load_bytes(
    space: &mut Space,
    dst_virt: usize,
    src: *const u8,
    size: usize,
    flags: usize,
) -> Result<(), Errno> {
    let mut off = 0usize;
    let mut rem = size;

    // TODO unaligned loads
    assert!(dst_virt & 0xFFF == 0);

    while rem != 0 {
        let page_idx = off / mem::PAGE_SIZE;
        let page_off = off % mem::PAGE_SIZE;
        let count = core::cmp::min(rem, mem::PAGE_SIZE - page_off);

        let page = phys::alloc_page(PageUsage::Kernel)?;
        let mut dst_flags = MapAttributes::NOT_GLOBAL | MapAttributes::SH_OUTER;

        if flags & (1 << 0) /* PF_X */ == 0 {
            dst_flags |= MapAttributes::UXN | MapAttributes::PXN;
        }

        match (flags & (3 << 1)) >> 1 {
            // No access: not sure if such mapping should exist at all
            0 => todo!(),
            // Write-only: not sure if such mapping should exist at all
            1 => todo!(),
            // Read-only
            2 => dst_flags |= MapAttributes::AP_BOTH_READONLY,
            // Read+Write
            3 => {}
            _ => unreachable!(),
        };

        debugln!(
            "Mapping {:#x} {:?}",
            dst_virt + page_idx * mem::PAGE_SIZE,
            dst_flags
        );
        space.map(dst_virt + page_idx * mem::PAGE_SIZE, page, dst_flags)?;

        let dst =
            core::slice::from_raw_parts_mut(mem::virtualize(page + page_off) as *mut u8, count);
        let src = core::slice::from_raw_parts(src.add(off), count);

        dst.copy_from_slice(src);

        rem -= count;
        off += count;
    }

    Ok(())
}

unsafe fn zero_bytes(
    space: &mut Space,
    dst_virt: usize,
    size: usize,
    flags: usize,
) -> Result<(), Errno> {
    let mut off = 0usize;
    let mut rem = size;

    while rem != 0 {
        let page_idx = (dst_virt + off - (dst_virt & !0xFFF)) / mem::PAGE_SIZE;
        let page_off = (dst_virt + off) % mem::PAGE_SIZE;
        let count = core::cmp::min(rem, mem::PAGE_SIZE - page_off);

        let page = phys::alloc_page(PageUsage::Kernel)?;
        let mut dst_flags = MapAttributes::NOT_GLOBAL | MapAttributes::SH_OUTER;

        if flags & (1 << 0) /* PF_X */ == 0 {
            dst_flags |= MapAttributes::UXN | MapAttributes::PXN;
        }

        match (flags & (3 << 1)) >> 1 {
            // No access: not sure if such mapping should exist at all
            0 => todo!(),
            // Write-only: not sure if such mapping should exist at all
            1 => todo!(),
            // Read-only
            2 => dst_flags |= MapAttributes::AP_BOTH_READONLY,
            // Read+Write
            3 => {}
            _ => unreachable!(),
        };

        debugln!(
            "Mapping {:#x} {:?}",
            dst_virt + page_idx * mem::PAGE_SIZE,
            dst_flags
        );
        if let Err(e) = space.map(dst_virt + page_idx * mem::PAGE_SIZE, page, dst_flags) {
            if e != Errno::AlreadyExists {
                return Err(e);
            }
        }

        let dst =
            core::slice::from_raw_parts_mut(mem::virtualize(page + page_off) as *mut u8, count);
        dst.fill(0);

        rem -= count;
        off += count;
    }

    Ok(())
}

///
pub fn load_elf(space: &mut Space, elf_base: *const u8) -> Result<usize, Errno> {
    let ehdr: &Ehdr<Elf64> = unsafe { &*(elf_base as *const _) };

    if &ehdr.ident[0..4] != b"\x7FELF" {
        return Err(Errno::InvalidArgument);
    }

    for i in 0..(ehdr.phnum as usize) {
        let phdr: &Phdr<Elf64> = unsafe {
            &*(elf_base.add(ehdr.phoff as usize + ehdr.phentsize as usize * i) as *const _)
        };

        if phdr.typ == 1
        /* PT_LOAD */
        {
            debugln!(
                "Load region {:#x}..{:#x}..{:#x}",
                phdr.vaddr,
                phdr.vaddr + phdr.filesz,
                phdr.vaddr + phdr.memsz
            );

            if phdr.filesz > 0 {
                unsafe {
                    load_bytes(
                        space,
                        phdr.vaddr as usize,
                        elf_base.add(phdr.offset as usize),
                        phdr.filesz as usize,
                        phdr.flags as usize,
                    )?;
                }
            }

            if phdr.memsz > phdr.filesz {
                let len = (phdr.memsz - phdr.filesz) as usize;
                unsafe {
                    zero_bytes(
                        space,
                        phdr.vaddr as usize + phdr.filesz as usize,
                        len,
                        phdr.flags as usize,
                    )?;
                }
            }
        }
    }

    Ok(ehdr.entry as usize)
}
