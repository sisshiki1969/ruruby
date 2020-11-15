use crate::*;
use std::fs::*;
use std::io::Read;
use std::path::PathBuf;

pub enum LoadError {
    NotFound(String),
    CouldntOpen(String),
}

pub fn load_file(path: &PathBuf) -> Result<String, LoadError> {
    let mut file_body = String::new();
    match OpenOptions::new().read(true).open(path) {
        Ok(mut file) => match file.read_to_string(&mut file_body) {
            Ok(_) => {}
            Err(ioerr) => {
                let msg = format!("{}", ioerr);
                return Err(LoadError::CouldntOpen(msg));
            }
        },
        Err(ioerr) => {
            let msg = format!("{}", ioerr);
            return Err(LoadError::CouldntOpen(msg));
        }
    };

    Ok(file_body)
}

/// Load file and execute.
pub fn load_exec(vm: &mut VM, path: &PathBuf, allow_repeat: bool) -> Result<bool, RubyError> {
    let absolute_path = vm.canonicalize_path(path)?;
    let res = vm.globals.add_source_file(&absolute_path);
    if !allow_repeat && res.is_none() {
        return Ok(false);
    }
    let program = vm.load_file(&absolute_path)?;
    #[cfg(feature = "verbose")]
    eprintln!("reading:{}", absolute_path.to_string_lossy());
    vm.class_push(BuiltinClass::object());
    vm.run(absolute_path, &program)?;
    vm.class_pop();
    Ok(true)
}
