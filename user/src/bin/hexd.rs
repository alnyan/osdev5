#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use libusr::io::{self, Read};
use libusr::file::File;

fn line_print(off: usize, line: &[u8]) {
    print!("{:08x}: ", off);
    for i in 0..16 {
        if i < line.len() {
            print!("{:02x}", line[i]);
        } else {
            print!("  ");
        }
        if i % 2 != 0 {
            print!(" ");
        }
    }
    print!("| ");
    for &b in line.iter() {
        if b.is_ascii() && !b.is_ascii_control() {
            print!("{}", b as char);
        } else {
            print!(".");
        }
    }
    println!("");
}

fn do_hexd<F: Read>(mut fd: F) -> Result<(), io::Error> {
    let mut buf = [0; 16];
    let mut off = 0;
    loop {
        let count = fd.read(&mut buf)?;
        if count == 0 {
            break;
        }

        line_print(off, &buf[..count]);
        off += count;
    }

    Ok(())
}

#[no_mangle]
fn main() -> i32 {
    let args = libusr::env::args();
    let mut res = 0;

    if args.len() == 1 {
        if let Err(e) = do_hexd(io::stdin()) {
            eprintln!("{}: {:?}", ".", e);
            res = -1;
        }
    } else {
        for arg in &args[1..] {
            if let Err(e) = File::open(arg).map(do_hexd) {
                eprintln!("{}: {:?}", arg, e);
                res = -1;
            }
        }
    }

    res
}
