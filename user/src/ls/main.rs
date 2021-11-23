#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;
#[macro_use]
extern crate alloc;

use libusr::sys::{sys_readdir, sys_openat, sys_close, sys_fstatat, stat::{FileMode, OpenFlags, DirectoryEntry, Stat}};
use alloc::{string::String, borrow::ToOwned};

#[no_mangle]
fn main() -> i32 {
    let mut buffer = [DirectoryEntry::empty(); 16];
    let mut stat = Stat::default();
    let mut data = vec![];

    let fd = sys_openat(None, "/", FileMode::default_dir(), OpenFlags::O_DIRECTORY | OpenFlags::O_RDONLY).unwrap();

    loop {
        let count = sys_readdir(fd, &mut buffer).unwrap();
        if count == 0 {
            break;
        }

        buffer.iter().take(count).for_each(|e| data.push(e.as_str().to_owned()));
    }

    data.sort();

    data.iter().for_each(|item| {
        let stat = sys_fstatat(Some(fd), item, &mut stat, 0).map(|_| &stat);
        if let Ok(stat) = stat {
            print!("{} ", stat.mode);
        } else {
            print!("?????????? ");
        }
        println!("{}", item);
    });

    sys_close(fd).unwrap();


    0
}
