use crate::*;
use std::fs::*;
use std::io::Read;
use std::path::{Path, PathBuf};

impl VM {
    pub fn load_file(&mut self, absolute_path: &Path) -> Result<String, RubyError> {
        match load_file(absolute_path) {
            Ok(program) => {
                self.globals.add_source_file(absolute_path);
                Ok(program)
            }
            Err(err) => Err(RubyError::load(format!(
                "Cannot open file. '{:?}'\n{}",
                absolute_path, err
            ))),
        }
    }

    pub(crate) fn require(&mut self, file_name: &str) -> Result<bool, RubyError> {
        let path = PathBuf::from(file_name);
        if path.is_absolute() {
            if let Some(res) = self.require_sub(path) {
                return res;
            }
        }
        let load_path = match self.get_global_var(IdentId::get_id("$:")) {
            Some(path) => path,
            None => return Ok(false),
        };
        let mut ainfo = load_path.expect_array("LOAD_PATH($:)")?;
        for path in ainfo.iter_mut() {
            let mut base_path = PathBuf::from(path.expect_string("LOAD_PATH($:)")?);
            base_path.push(file_name);
            if let Some(res) = self.require_sub(base_path) {
                return res;
            }
        }
        Err(RubyError::load(format!(
            "Can not load such file -- {:?}",
            file_name
        )))
    }

    fn require_sub(&mut self, mut path: PathBuf) -> Option<Result<bool, RubyError>> {
        path.set_extension("rb");
        if path.exists() {
            return Some(self.load_exec(&path, false));
        }
        path.set_extension("so");
        if path.exists() {
            unsafe {
                let lib = match libloading::Library::new(&path) {
                    Ok(lib) => lib,
                    Err(err) => return Some(Err(RubyError::load(err.to_string()))),
                };
                let fn_name = format!("Init_{}", path.file_stem().unwrap().to_string_lossy());
                let func: libloading::Symbol<unsafe extern "C" fn()> =
                    match lib.get(&fn_name.as_bytes()) {
                        Ok(sym) => sym,
                        Err(err) => return Some(Err(RubyError::load(err.to_string()))),
                    };
                eprintln!("load so: {}", fn_name);
                //func();
                return Some(Ok(false));
            }
        }
        None
    }

    /// Load file and execute.
    /// returns Ok(true) if the file was actually loaded and executed.
    /// otherwise, returns Ok(false).
    pub(crate) fn load_exec(&mut self, path: &Path, allow_repeat: bool) -> Result<bool, RubyError> {
        let absolute_path = match path.canonicalize() {
            Ok(path) => path,
            Err(ioerr) => {
                let msg = format!("File not found. {:?}\n{}", path, ioerr);
                return Err(RubyError::runtime(msg));
            }
        };
        let res = self.globals.add_source_file(&absolute_path);
        if !allow_repeat && res.is_none() {
            return Ok(false);
        }
        let program = self.load_file(&absolute_path)?;
        self.run(absolute_path, program)?;
        Ok(true)
    }
}

fn load_file(path: &Path) -> Result<String, String> {
    let mut file_body = String::new();
    match OpenOptions::new().read(true).open(path) {
        Ok(mut file) => match file.read_to_string(&mut file_body) {
            Ok(_) => {}
            Err(ioerr) => return Err(format!("{}", ioerr)),
        },
        Err(ioerr) => return Err(format!("{}", ioerr)),
    };

    Ok(file_body)
}
