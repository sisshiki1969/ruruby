use crate::*;
use std::fs::*;
use std::io::Read;
use std::path::PathBuf;

impl VM {
    pub fn load_file(&mut self, absolute_path: &PathBuf) -> Result<String, RubyError> {
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

    pub fn require(&mut self, file_name: &str) -> Result<bool, RubyError> {
        let mut path = PathBuf::from(file_name);
        if path.is_absolute() {
            path.set_extension("rb");
            if path.exists() {
                return self.load_exec(&path, false);
            }
            path.set_extension("so");
            if path.exists() {
                eprintln!("Warning: currently, can not require .so file. {:?}", path);
                return Ok(false);
            }
        }
        let mut load_path = match self.get_global_var(IdentId::get_id("$:")) {
            Some(path) => path,
            None => return Ok(false),
        };
        let mut ainfo = load_path.expect_array("LOAD_PATH($:)")?;
        for path in ainfo.iter_mut() {
            let mut base_path = PathBuf::from(path.expect_string("LOAD_PATH($:)")?);
            base_path.push(file_name);
            base_path.set_extension("rb");
            if base_path.exists() {
                return self.load_exec(&base_path, false);
            }
            base_path.set_extension("so");
            if base_path.exists() {
                eprintln!(
                    "Warning: currently, can not require .so file. {:?}",
                    base_path
                );
                return Ok(false);
            }
        }
        Err(RubyError::load(format!(
            "Can not load such file -- {:?}",
            file_name
        )))
    }

    /// Load file and execute.
    /// returns Ok(true) if the file was actually loaded and executed.
    /// otherwise, returns Ok(false).
    pub fn load_exec(&mut self, path: &PathBuf, allow_repeat: bool) -> Result<bool, RubyError> {
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

pub fn load_file(path: &PathBuf) -> Result<String, String> {
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
