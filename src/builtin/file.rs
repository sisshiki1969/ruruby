use crate::*;
use std::fs::File;
use std::io::Read;
use std::path::*;

pub fn init(_globals: &mut Globals) -> Value {
    let class = ClassRef::from_str("File", BuiltinClass::object());
    let mut class_val = Value::class(class);
    class_val.add_builtin_class_method("join", join);
    class_val.add_builtin_class_method("basename", basename);
    class_val.add_builtin_class_method("extname", extname);
    class_val.add_builtin_class_method("dirname", dirname);
    class_val.add_builtin_class_method("binread", binread);
    class_val.add_builtin_class_method("read", read);
    class_val.add_builtin_class_method("write", write);
    class_val.add_builtin_class_method("expand_path", expand_path);
    class_val.add_builtin_class_method("exist?", exist);
    class_val.add_builtin_class_method("directory?", directory);
    class_val.add_builtin_class_method("file?", file);
    class_val.add_builtin_class_method("realpath", realpath);
    class_val
}

// Utils

/// Convert Ruby String value`string` to PathBuf.
fn string_to_path(vm: &mut VM, mut string: Value, msg: &str) -> Result<PathBuf, RubyError> {
    let file = string.expect_string(vm, msg)?;
    Ok(PathBuf::from(file))
}

/// Convert Ruby String value`string` to canonicalized PathBuf.
fn string_to_canonicalized_path(
    vm: &mut VM,
    string: Value,
    msg: &str,
) -> Result<PathBuf, RubyError> {
    match string_to_path(vm, string, msg)?.canonicalize() {
        Ok(file) => Ok(file),
        Err(_) => Err(vm.error_argument(format!("{} is an invalid filename. {:?}", msg, string))),
    }
}

// Class methods

fn join(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2)?;
    let mut path = string_to_path(vm, args[0], "1st agr")?;
    let arg = string_to_path(vm, args[1], "2nd arg")?;

    for p in arg.iter() {
        if p == ".." {
            path.pop();
        } else {
            path.push(p);
        }
    }
    Ok(Value::string(path.to_string_lossy().to_string()))
}

fn basename(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_range(len, 1, 1)?;
    let filename = string_to_path(vm, args[0], "1st arg")?;
    let basename = match filename.file_name() {
        Some(ostr) => Value::string(ostr.to_string_lossy().to_string()),
        None => Value::nil(),
    };
    Ok(basename)
}

fn extname(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_range(len, 1, 1)?;
    let filename = string_to_path(vm, args[0], "1st arg")?;
    let extname = match filename.extension() {
        Some(ostr) => format!(".{}", ostr.to_string_lossy()),
        None => "".to_string(),
    };
    Ok(Value::string(extname))
}

fn dirname(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_range(len, 1, 1)?;
    let filename = string_to_path(vm, args[0], "1st arg")?;
    let dirname = match filename.parent() {
        Some(ostr) => format!("{}", ostr.to_string_lossy()),
        None => "".to_string(),
    };
    Ok(Value::string(dirname))
}

fn binread(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_range(len, 1, 1)?;
    let filename = string_to_canonicalized_path(vm, args[0], "1st arg")?;
    let mut file = match File::open(&filename) {
        Ok(file) => file,
        Err(_) => return Err(vm.error_internal(format!("Can not open file. {:?}", &filename))),
    };
    let mut contents = vec![];
    match file.read_to_end(&mut contents) {
        Ok(file) => file,
        Err(_) => return Err(vm.error_internal("Could not read the file.")),
    };
    Ok(Value::bytes(contents))
}

/// IO.read(path)
fn read(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_range(len, 1, 1)?;
    let filename = string_to_path(vm, args[0], "1st arg")?;
    let mut file = match File::open(&filename) {
        Ok(file) => file,
        Err(_) => return Err(vm.error_internal(format!("Can not open file. {:?}", &filename))),
    };
    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
        Ok(file) => file,
        Err(_) => return Err(vm.error_internal("Could not read the file.")),
    };
    Ok(Value::string(contents))
}

/// IO.write(path, string)
fn write(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_num(len, 2)?;
    let mut arg0 = args[0];
    let mut arg1 = args[1];
    let filename = arg0.expect_string(vm, "1st arg")?;
    let contents = arg1.expect_string(vm, "2nd arg")?;
    match std::fs::write(&filename, contents) {
        Ok(()) => {}
        Err(err) => {
            return Err(vm.error_internal(format!(
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
    vm.check_args_range(len, 1, 2)?;
    let current_dir = std::env::current_dir()
        .or_else(|_| Err(vm.error_internal("Failed to get current directory.")))?;
    let home_dir = dirs::home_dir().ok_or(vm.error_internal("Failed to get home directory."))?;
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

    return Ok(Value::string(res_path.to_string_lossy().to_string()));
}
fn exist(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 1, 1)?;
    let b = string_to_canonicalized_path(vm, args[0], "1st arg").is_ok();
    Ok(Value::bool(b))
}

fn directory(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 1, 1)?;
    let b = match string_to_canonicalized_path(vm, args[0], "1st arg") {
        Ok(path) => path.is_dir(),
        Err(_) => false,
    };
    Ok(Value::bool(b))
}

fn file(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 1, 1)?;
    let b = match string_to_canonicalized_path(vm, args[0], "1st arg") {
        Ok(path) => path.is_file(),
        Err(_) => false,
    };
    Ok(Value::bool(b))
}

fn realpath(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 1, 1)?;
    let b = string_to_canonicalized_path(vm, args[0], "1st arg")?;
    Ok(Value::string(b.to_string_lossy().to_string()))
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn file() {
        let program = r###"
            File.write("file.txt","foo")
            assert("foo", File.read("file.txt"))
            File.write("file.txt","bar")
            assert("bar", File.read("file.txt"))
            assert("file.txt", File.basename("/home/usr/file.txt"))
            assert(".txt", File.extname("/home/usr/file.txt"))
            assert("/home/usr", File.dirname("/home/usr/file.txt"))
            assert(true, File.exist? "Cargo.toml")
            assert(false, File.exist? "Cargo.tomlz")
            assert(true, File.directory? "src")
            assert(false, File.directory? "srcs")
            assert(false, File.directory? "Cargo.toml")
            assert(false, File.file? "src")
            assert(true, File.file? "Cargo.toml")
            assert(false, File.file? "Cargo.tomlz")
            assert("#{Dir.pwd}/Cargo.toml", File.realpath "Cargo.toml")
            assert_error { File.realpath "Cargo.tomlz" }
        "###;
        assert_script(program);
    }

    #[test]
    fn file_expand_path() {
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
        assert_script(program);
    }
}
