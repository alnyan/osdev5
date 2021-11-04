//! Executable and Linkable Format binary loader module
use crate::mem::{
    self,
    phys::{self, PageUsage},
    virt::{MapAttributes, Space},
};
use core::mem::{size_of, MaybeUninit};
use error::Errno;
use libcommon::{Read, Seek, SeekDir};

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

fn map_flags(elf_flags: usize) -> MapAttributes {
    let mut dst_flags = MapAttributes::NOT_GLOBAL | MapAttributes::SH_OUTER;

    if elf_flags & (1 << 0) /* PF_X */ == 0 {
        dst_flags |= MapAttributes::UXN | MapAttributes::PXN;
    }

    match (elf_flags & (3 << 1)) >> 1 {
        // No access: not sure if such mapping should exist at all
        0 => todo!(),
        // Write-only: not sure if such mapping should exist at all
        1 => todo!(),
        // Read-only
        2 => dst_flags |= MapAttributes::AP_BOTH_READONLY,
        // Read+Write
        3 => dst_flags |= MapAttributes::AP_BOTH_READWRITE,
        _ => unreachable!(),
    };

    dst_flags
}

unsafe fn load_bytes<F>(
    space: &mut Space,
    dst_virt: usize,
    mut read: F,
    size: usize,
    flags: usize,
) -> Result<(), Errno>
where
    F: FnMut(usize, &mut [u8]) -> Result<(), Errno>,
{
    let dst_page_off = dst_virt & 0xFFF;
    let dst_page = dst_virt & !0xFFF;
    let mut off = 0usize;
    let mut rem = size;

    while rem != 0 {
        let page_idx = (dst_page_off + off) / mem::PAGE_SIZE;
        let page_off = (dst_page_off + off) % mem::PAGE_SIZE;
        let count = core::cmp::min(rem, mem::PAGE_SIZE - page_off);

        let page = phys::alloc_page(PageUsage::Kernel)?;

        // TODO fetch existing mapping and test flag equality instead
        //      if flags differ, bail out
        if let Err(e) = space.map(dst_page + page_idx * mem::PAGE_SIZE, page, map_flags(flags)) {
            if e != Errno::AlreadyExists {
                return Err(e);
            }
        }

        let dst_page_virt = mem::virtualize(page + page_off);
        let dst = core::slice::from_raw_parts_mut(dst_page_virt as *mut u8, count);

        read(off, dst)?;

        rem -= count;
        off += count;
    }

    Ok(())
}

unsafe fn read_struct<T, F: Seek + Read>(src: &mut F, pos: usize) -> Result<T, Errno> {
    let mut storage: MaybeUninit<T> = MaybeUninit::uninit();
    let size = size_of::<T>();
    src.seek(pos as isize, SeekDir::Set)?;
    let res = src.read(core::slice::from_raw_parts_mut(
        storage.as_mut_ptr() as *mut u8,
        size,
    ))?;
    if res != size {
        Err(Errno::InvalidFile)
    } else {
        Ok(storage.assume_init())
    }
}

/// Loads an ELF program from `source` into target `space`
pub fn load_elf<F: Seek + Read>(space: &mut Space, source: &mut F) -> Result<usize, Errno> {
    let ehdr: Ehdr<Elf64> = unsafe { read_struct(source, 0).unwrap() };

    if &ehdr.ident[0..4] != b"\x7FELF" {
        return Err(Errno::BadExecutable);
    }

    for i in 0..(ehdr.phnum as usize) {
        let phdr: Phdr<Elf64> = unsafe {
            read_struct(source, ehdr.phoff as usize + ehdr.phentsize as usize * i).unwrap()
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
                        |off, dst| {
                            source.seek(phdr.offset as isize + off as isize, SeekDir::Set)?;
                            if source.read(dst)? == dst.len() {
                                Ok(())
                            } else {
                                Err(Errno::InvalidFile)
                            }
                        },
                        phdr.filesz as usize,
                        phdr.flags as usize,
                    )?;
                }
            }

            if phdr.memsz > phdr.filesz {
                let len = (phdr.memsz - phdr.filesz) as usize;
                unsafe {
                    load_bytes(
                        space,
                        phdr.vaddr as usize + phdr.filesz as usize,
                        |_, dst| {
                            dst.fill(0);
                            Ok(())
                        },
                        len,
                        phdr.flags as usize,
                    )?;
                }
            }
        }
    }

    Ok(ehdr.entry as usize)
}
