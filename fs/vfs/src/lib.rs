//! Virtual filesystem API and facilities
#![warn(missing_docs)]
#![feature(const_fn_trait_bound, const_discriminant)]
#![no_std]

#[cfg(test)]
#[macro_use]
extern crate std;

extern crate alloc;

// pub use libsys::stat::{FileMode, OpenFlags, Stat};
// pub use libsys::ioctl::IoctlCmd;

mod block;
pub use block::BlockDevice;
mod fs;
pub use fs::Filesystem;
mod node;
pub use node::{
    Vnode, VnodeCommon, VnodeCreateKind, VnodeData, VnodeDirectory, VnodeFile, VnodeRef,
};
mod ioctx;
pub use ioctx::Ioctx;
mod file;
pub use file::{File, FileRef};
mod char;
pub use crate::char::CharDevice;
