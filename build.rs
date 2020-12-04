use std::process::Command;
use std::env;
use std::fs;
use std::path::Path;

fn main(){
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("libpath.rb");
  let load_path = match Command::new("ruby").args(&["-e", "p($:)"]).output() {
      Ok(output) => match std::str::from_utf8(&output.stdout) {
          Ok(s) => s.to_string(),
          Err(_) => "[]".to_string(),
      },
      Err(_) => "[]".to_string(),
  };
  fs::write(
    &dest_path,
    load_path,
  ).unwrap();
}