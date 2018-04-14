use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    Command::new("nasm")
        .args(&["src/cn_aesni.asm", "-f", "elf64", "-g", "-o"])
        .arg(&format!("{}/cnaesni.o", out_dir))
        .status()
        .unwrap();
    Command::new("ar")
        .args(&["crus", "libcnaesni.a", "cnaesni.o"])
        .current_dir(&Path::new(&out_dir))
        .status()
        .unwrap();

    println!("cargo:rustc-link-search=native={}", out_dir);
    println!("cargo:rustc-link-lib=static=cnaesni");
}
