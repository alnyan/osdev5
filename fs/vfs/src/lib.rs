#![feature(destructuring_assignment)]
#![no_std]

extern crate alloc;

pub mod block;
pub use block::BlockDevice;
pub mod fs;
pub use fs::Filesystem;
pub mod stat;
pub use stat::FileMode;
pub mod node;
pub use node::{Vnode, VnodeImpl, VnodeKind, VnodeRef};
pub mod ioctx;
pub use ioctx::Ioctx;
pub mod file;
pub use file::File;
