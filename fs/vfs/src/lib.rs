//! Virtual filesystem API and facilities
#![warn(missing_docs)]
#![feature(destructuring_assignment, const_fn_trait_bound)]
#![no_std]

#[cfg(test)]
#[macro_use]
extern crate std;

#[macro_use]
extern crate fs_macros;

extern crate alloc;

// pub use libsys::stat::{FileMode, OpenFlags, Stat};
// pub use libsys::ioctl::IoctlCmd;

mod block;
pub use block::BlockDevice;
mod fs;
pub use fs::Filesystem;
mod node;
pub use node::{Vnode, VnodeImpl, VnodeKind, VnodeRef};
mod ioctx;
pub use ioctx::Ioctx;
mod file;
pub use file::{File, FileRef};
mod char;
pub use crate::char::{CharDevice, CharDeviceWrapper};
