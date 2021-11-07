use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("libpath.rb");
    let output = Command::new("ruby").args(&["-e", "p($:)"]).output();
    let load_path = match &output {
        Ok(output) => std::str::from_utf8(&output.stdout).unwrap_or("[]"),
        Err(_) => "[]",
    };
    fs::write(&dest_path, load_path).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
