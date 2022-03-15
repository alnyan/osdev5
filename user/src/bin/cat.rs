#![no_std]
#![no_main]

#[macro_use]
extern crate libusr;

use libusr::file::File;
use libusr::io::{self, Read, Write};

fn do_cat<F: Read>(mut fd: F) -> Result<(), io::Error> {
    let mut buf = [0; 4096];
    let mut out = io::stdout();

    loop {
        let count = fd.read(&mut buf)?;
        if count == 0 {
            break;
        }

        out.write(&buf[..count])?;
    }

    Ok(())
}

#[no_mangle]
fn main() -> i32 {
    let args = libusr::env::args();
    let mut res = 0;

    if args.len() == 1 {
        if let Err(e) = do_cat(io::stdin()) {
            eprintln!(".: {:?}", e);
            res = -1;
        }
    } else {
        for arg in &args[1..] {
            if let Err(e) = File::open(arg).map(do_cat) {
                eprintln!("{}: {:?}", arg, e);
                res = -1;
            }
        }
    }

    res
}
