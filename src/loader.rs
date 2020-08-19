use std::fs::*;
use std::io::Read;

pub enum LoadError {
    NotFound(String),
    CouldntOpen(String),
}

pub fn load_file(file_name: &str) -> Result<(std::path::PathBuf, String), LoadError> {
    let path = std::path::Path::new(file_name); //.with_extension("rb");
    let absolute_path = match path.canonicalize() {
        Ok(path) => path,
        Err(ioerr) => {
            let msg = format!("{}", ioerr);
            return Err(LoadError::NotFound(msg));
        }
    };
    let mut file_body = String::new();
    match OpenOptions::new().read(true).open(&absolute_path) {
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

    Ok((absolute_path, file_body))
}
