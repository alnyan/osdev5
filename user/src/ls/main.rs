#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;
#[macro_use]
extern crate alloc;

use alloc::{borrow::ToOwned, string::String};
use libusr::sys::{
    stat::{DirectoryEntry, FileMode, OpenFlags, Stat},
    sys_close, sys_fstatat, sys_openat, sys_readdir, Errno,
};

fn list_directory(path: &str) -> Result<(), Errno> {
    let mut buffer = [DirectoryEntry::empty(); 8];
    let mut stat = Stat::default();
    let mut data = vec![];

    let fd = sys_openat(
        None,
        path,
        FileMode::default_dir(),
        OpenFlags::O_DIRECTORY | OpenFlags::O_RDONLY,
    )?;

    loop {
        let count = sys_readdir(fd, &mut buffer)?;
        if count == 0 {
            break;
        }

        buffer.iter().take(count).for_each(|e| {
            data.push(e.as_str().to_owned());
        });
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

    sys_close(fd)
}

#[no_mangle]
fn main() -> i32 {
    let args = libusr::env::args();
    let mut res = 0;

    if args.len() == 1 {
        if let Err(e) = list_directory(".") {
            eprintln!("{}: {:?}", ".", e);
            res = -1;
        }
    } else {
        for arg in &args[1..] {
            if let Err(e) = list_directory(arg) {
                eprintln!("{}: {:?}", arg, e);
                res = -1;
            }
        }
    }

    res
}
