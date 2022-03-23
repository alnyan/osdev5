#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use libsys::stat::FileMode;
use libusr::sys::sys_mkdirat;

#[no_mangle]
fn main() -> i32 {
    let args = libusr::env::args();

    if args.len() < 2 {
        eprintln!("Usage: {} DIR1 ...", args[0]);
        return -1;
    }

    let mut status = 0;
    for &item in args.iter().skip(1) {
        if let Err(err) = sys_mkdirat(None, item, FileMode::default_dir(), 0) {
            eprintln!("{}: {:?}", item, err);
            status = -1;
        }
    }
    status
}
