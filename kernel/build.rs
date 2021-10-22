use std::env;

fn main() -> Result<(), i32> {
    let arch = env::var("ARCH").expect("$ARCH is not set");
    let mach = if arch == "aarch64" {
        env::var("MACH").expect("$MACH is not set")
    } else {
        "none".to_owned()
    };
    println!("cargo:rerun-if-changed=../etc/{}-{}.ld", arch, mach);
    Ok(())
}
