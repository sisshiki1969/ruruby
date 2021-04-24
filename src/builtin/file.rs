use crate::*;
use std::fs::File;
use std::io::Read;
use std::path::*;

pub fn init() -> Value {
    let io_class = BuiltinClass::get_toplevel_constant("IO").unwrap();
    let class = Module::class_under(Module::new(io_class));
    BuiltinClass::set_toplevel_constant("File", class);
    class.add_builtin_class_method("join", join);
    class.add_builtin_class_method("basename", basename);
    class.add_builtin_class_method("extname", extname);
    class.add_builtin_class_method("dirname", dirname);
    class.add_builtin_class_method("binread", binread);
    class.add_builtin_class_method("read", read);
    class.add_builtin_class_method("readlines", readlines);
    class.add_builtin_class_method("write", write);
    class.add_builtin_class_method("expand_path", expand_path);
    class.add_builtin_class_method("exist?", exist);
    class.add_builtin_class_method("executable?", executable);
    class.add_builtin_class_method("directory?", directory);
    class.add_builtin_class_method("file?", file);
    class.add_builtin_class_method("realpath", realpath);
    class.into()
}

// Utils

/// Convert Ruby String value`string` to PathBuf.
fn string_to_path(_: &mut VM, mut string: Value, msg: &str) -> Result<PathBuf, RubyError> {
    let file = string.expect_string(msg)?;
    let mut path = PathBuf::new();
    for p in PathBuf::from(file).iter() {
        if p == ".." && path.file_name().is_some() {
            path.pop();
        } else {
            path.push(p);
        };
    }
    Ok(path)
}

/// Canonicalize PathBuf.
fn canonicalize_path(_: &mut VM, path: PathBuf) -> Result<PathBuf, RubyError> {
    match path.canonicalize() {
        Ok(file) => Ok(file),
        Err(_) => Err(RubyError::argument(format!(
            "Invalid file path. {:?}",
            path
        ))),
    }
}

/// Convert Ruby String value`string` to canonicalized PathBuf.
pub fn string_to_canonicalized_path(
    vm: &mut VM,
    string: Value,
    msg: &str,
) -> Result<PathBuf, RubyError> {
    let path = string_to_path(vm, string, msg)?;
    match path.canonicalize() {
        Ok(file) => Ok(file),
        Err(_) => Err(RubyError::argument(format!(
            "{} is an invalid filename. {:?}",
            msg, path
        ))),
    }
}

// Class methods

fn join(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    fn flatten(vm: &mut VM, path: &mut String, mut val: Value) -> Result<(), RubyError> {
        match val.as_array() {
            Some(ainfo) => {
                for v in ainfo.elements.iter() {
                    flatten(vm, path, *v)?;
                }
            }
            None => {
                if !path.is_empty() && !path.ends_with('/') {
                    path.push('/');
                }
                let s = val.expect_string("Args")?;
                path.push_str(if !path.is_empty() && s.starts_with('/') {
                    &s[1..]
                } else {
                    s
                });
            }
        }
        Ok(())
    }
    let mut path = String::new();
    for arg in args.iter() {
        flatten(vm, &mut path, *arg)?;
    }
    Ok(Value::string(path))
}

fn basename(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 1)?;
    let filename = string_to_path(vm, args[0], "1st arg")?;
    let basename = match filename.file_name() {
        Some(ostr) => Value::string(ostr.to_string_lossy()),
        None => Value::nil(),
    };
    Ok(basename)
}

fn extname(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 1)?;
    let filename = string_to_path(vm, args[0], "1st arg")?;
    let extname = match filename.extension() {
        Some(ostr) => format!(".{}", ostr.to_string_lossy()),
        None => "".to_string(),
    };
    Ok(Value::string(extname))
}

fn dirname(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 1)?;
    let filename = string_to_path(vm, args[0], "1st arg")?;
    let dirname = match filename.parent() {
        Some(ostr) => format!("{}", ostr.to_string_lossy()),
        None => "".to_string(),
    };
    Ok(Value::string(dirname))
}

fn binread(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 1)?;
    let filename = string_to_canonicalized_path(vm, args[0], "1st arg")?;
    let mut file = match File::open(&filename) {
        Ok(file) => file,
        Err(_) => {
            return Err(RubyError::internal(format!(
                "Can not open file. {:?}",
                &filename
            )))
        }
    };
    let mut contents = vec![];
    match file.read_to_end(&mut contents) {
        Ok(file) => file,
        Err(_) => return Err(RubyError::internal("Could not read the file.")),
    };
    Ok(Value::bytes(contents))
}

/// IO.read(path)
fn read(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let filename = string_to_path(vm, args[0], "1st arg")?;
    let mut file = match File::open(&filename) {
        Ok(file) => file,
        Err(_) => {
            return Err(RubyError::internal(format!(
                "Can not open file. {:?}",
                &filename
            )))
        }
    };
    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
        Ok(file) => file,
        Err(_) => return Err(RubyError::internal("Could not read the file.")),
    };
    Ok(Value::string(contents))
}

/// IO.readlines(path)
fn readlines(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let filename = string_to_path(vm, args[0], "1st arg")?;
    let mut file = match File::open(&filename) {
        Ok(file) => file,
        Err(_) => {
            return Err(RubyError::internal(format!(
                "Can not open file. {:?}",
                &filename
            )))
        }
    };
    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
        Ok(file) => file,
        Err(_) => return Err(RubyError::internal("Could not read the file.")),
    };
    let ary = contents.split('\n').map(|s| Value::string(s)).collect();
    Ok(Value::array_from(ary))
}

/// IO.write(path, string)
fn write(_: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(2)?;
    let mut arg0 = args[0];
    let mut arg1 = args[1];
    let filename = arg0.expect_string("1st arg")?;
    let contents = arg1.expect_string("2nd arg")?;
    match std::fs::write(&filename, contents) {
        Ok(()) => {}
        Err(err) => {
            return Err(RubyError::internal(format!(
                "Can not create or write file. {:?}\n{:?}",
                &filename, err
            )))
        }
    };
    Ok(Value::integer(contents.len() as i64))
}

/// File.expand_path(path, default_dir = '.') -> String
fn expand_path(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    let len = args.len();
    args.check_args_range(1, 2)?;
    let current_dir = std::env::current_dir()
        .or_else(|_| Err(RubyError::internal("Failed to get current directory.")))?;
    let home_dir = dirs::home_dir().ok_or(RubyError::internal("Failed to get home directory."))?;
    let path = if len == 1 {
        string_to_path(vm, args[0], "1st arg")?
    } else {
        let mut path = string_to_path(vm, args[1], "2nd arg")?;
        let rel_path = string_to_path(vm, args[0], "1st arg")?;
        path.push(rel_path);
        path
    };

    let mut res_path = PathBuf::new();
    res_path.push(current_dir);

    for elem in path.components() {
        match elem {
            Component::CurDir => {}
            Component::Normal(comp) if comp == "~" => {
                res_path.clear();
                res_path.push(home_dir.clone());
            }
            Component::Normal(comp) => res_path.push(comp),
            Component::ParentDir => {
                res_path.pop();
            }
            Component::RootDir => {
                res_path.clear();
                res_path.push(Component::RootDir);
            }
            _ => {}
        };
    }
    //eprintln!("{:?}", res_path);

    return Ok(Value::string(res_path.to_string_lossy()));
}

fn exist(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 1)?;
    let b = string_to_canonicalized_path(vm, args[0], "1st arg").is_ok();
    Ok(Value::bool(b))
}

fn executable(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 1)?;
    let b = string_to_canonicalized_path(vm, args[0], "1st arg").is_ok();
    Ok(Value::bool(b))
}

fn directory(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 1)?;
    let b = match string_to_canonicalized_path(vm, args[0], "1st arg") {
        Ok(path) => path.is_dir(),
        Err(_) => false,
    };
    Ok(Value::bool(b))
}

fn file(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 1)?;
    let b = match string_to_canonicalized_path(vm, args[0], "1st arg") {
        Ok(path) => path.is_file(),
        Err(_) => false,
    };
    Ok(Value::bool(b))
}

fn realpath(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let mut pathname = args[0];
    let mut root = if args.len() == 2 {
        string_to_path(vm, args[1], "2nd arg")?
    } else {
        PathBuf::new()
    };
    root.push(pathname.expect_string("1st arg")?);
    let path_buf = canonicalize_path(vm, root)?;
    let path_str = path_buf.to_string_lossy();
    Ok(Value::string(path_str))
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn file() {
        let program = r###"
            File.write("file.txt","foo")
            assert "foo", File.read("file.txt")
            File.write("file.txt","foo\nbar\nboo")
            assert ["foo", "bar", "boo"], File.readlines("file.txt")
            File.write("file.txt","bar")
            assert "bar", File.read("file.txt")
            assert "file.txt", File.basename("/home/usr/file.txt")
            assert ".txt", File.extname("/home/usr/file.txt")
            assert "/home/usr", File.dirname("/home/usr/file.txt")
            assert true, File.exist? "Cargo.toml"
            assert false, File.exist? "Cargo.tomlz"
            assert true, File.directory? "src"
            assert false, File.directory? "srcs"
            assert false, File.directory? "Cargo.toml"
            assert false, File.file? "src"
            assert true, File.file? "Cargo.toml"
            assert false, File.file? "Cargo.tomlz"

            assert "#{Dir.pwd}/Cargo.toml", File.realpath "Cargo.toml"
            assert_error { File.realpath "Cargo.tomlz" }
            assert_error { File.realpath("Cargo.tomlz", "/") }

            assert "a/b/c", File.join("a","b","c")
            assert "a/b/c", File.join([["a"],"b"],"c")
            assert "a/b/c", File.join("a/","b","/c")
            assert "a/b/c", File.join("a/","/b/","/c")
            assert "/a/b/c", File.join("/a/","/b/","/c")
            assert "", File.join
            assert "", File.join([])
        "###;
        assert_script(program);
    }

    #[test]
    fn file_expand_path() {
        #[cfg(not(windows))]
        let program = r###"
            assert(Dir.pwd, File.expand_path("."))
            assert(Dir.pwd, File.expand_path("", "."))
            #assert("#{ENV["HOME"]}", File.expand_path(".."))
            #assert("#{ENV["HOME"]}", File.expand_path("..", "."))
            #assert("/home", File.expand_path("../.."))
            #assert("/home", File.expand_path("../..", "."))
            #assert("/home", File.expand_path("../../", "."))
            assert("/", File.expand_path("/"))
            assert(Dir.pwd, File.expand_path("../", "tests"))
            assert("/home", File.expand_path("home", "/"))
            assert(Dir.home, File.expand_path("#{ENV["HOME"]}", "/"))
            assert("/ruruby", File.expand_path("ruruby", "/"))
            assert(Dir.home, File.expand_path("~"))
        "###;
        #[cfg(windows)]
        let program = r###"
            assert(Dir.pwd, File.expand_path("."))
            assert(Dir.pwd, File.expand_path("", "."))
            #assert("#{ENV["HOME"]}", File.expand_path(".."))
            #assert("#{ENV["HOME"]}", File.expand_path("..", "."))
            #assert("C:/Users", File.expand_path("../.."))
            #assert("C:/Users", File.expand_path("../..", "."))
            #assert("C:/Users", File.expand_path("../../", "."))
            assert("C:/", File.expand_path("/"))
            assert(Dir.pwd, File.expand_path("../", "tests"))
            assert("C:/home", File.expand_path("home", "/"))
            assert(Dir.home, File.expand_path("#{ENV["HOME"]}", "/"))
            assert("C:/ruruby", File.expand_path("ruruby", "/"))
            assert(Dir.home, File.expand_path("~"))
        "###;
        assert_script(program);
    }
}
