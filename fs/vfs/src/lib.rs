//! Virtual filesystem API and facilities
#![warn(missing_docs)]
#![feature(destructuring_assignment, const_fn_trait_bound)]
#![no_std]

#[cfg(test)]
#[macro_use]
extern crate std;

extern crate alloc;

mod block;
pub use block::BlockDevice;
mod fs;
pub use fs::Filesystem;
mod stat;
pub use stat::FileMode;
mod node;
pub use node::{Vnode, VnodeImpl, VnodeKind, VnodeRef};
mod ioctx;
pub use ioctx::Ioctx;
mod file;
pub use file::File;
mod char;
pub use crate::char::{CharDevice, CharDeviceWrapper};
