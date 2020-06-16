use std::fs::File;
use std::io::Read;
use std::path::*;
//#[macro_use]
use crate::*;

pub fn init_file(globals: &mut Globals) -> Value {
    let id = IdentId::get_ident_id("File");
    let class = ClassRef::from(id, globals.builtins.object);
    let obj = Value::class(globals, class);
    globals.add_builtin_class_method(obj, "join", join);
    globals.add_builtin_class_method(obj, "basename", basename);
    globals.add_builtin_class_method(obj, "extname", extname);
    globals.add_builtin_class_method(obj, "binread", binread);
    globals.add_builtin_class_method(obj, "read", read);
    globals.add_builtin_class_method(obj, "write", write);
    obj
}

// Utils

fn string_to_path(vm: &mut VM, mut string: Value) -> Result<PathBuf, RubyError> {
    let file = string.expect_string(vm, "")?;
    Ok(PathBuf::from(file))
}

// Class methods

fn join(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2)?;
    let mut path = string_to_path(vm, args[0])?;
    let arg = string_to_path(vm, args[1])?;

    for p in arg.iter() {
        if p == ".." {
            path.pop();
        } else {
            path.push(p);
        }
    }
    Ok(Value::string(
        &vm.globals,
        path.to_string_lossy().to_string(),
    ))
}

fn basename(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_range(len, 1, 1)?;
    let filename = string_to_path(vm, args[0])?;
    let basename = match filename.file_name() {
        Some(ostr) => Value::string(&vm.globals, ostr.to_string_lossy().into_owned()),
        None => Value::nil(),
    };
    Ok(basename)
}

fn extname(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_range(len, 1, 1)?;
    let filename = string_to_path(vm, args[0])?;
    let extname = match filename.extension() {
        Some(ostr) => format!(".{}", ostr.to_string_lossy().into_owned()),
        None => "".to_string(),
    };
    Ok(Value::string(&vm.globals, extname))
}

fn binread(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_range(len, 1, 1)?;
    let filename = match string_to_path(vm, args[0])?.canonicalize() {
        Ok(file) => file,
        Err(_) => {
            let inspect = vm.val_inspect(args[0]);
            return Err(vm.error_argument(format!("Invalid filename. {}", inspect)));
        }
    };
    let mut file = match File::open(&filename) {
        Ok(file) => file,
        Err(_) => return Err(vm.error_internal(format!("Can not open file. {:?}", &filename))),
    };
    let mut contents = vec![];
    match file.read_to_end(&mut contents) {
        Ok(file) => file,
        Err(_) => return Err(vm.error_internal("Could not read the file.")),
    };
    Ok(Value::bytes(&vm.globals, contents))
}

/// IO.read(path)
fn read(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_range(len, 1, 1)?;
    let filename = match string_to_path(vm, args[0])?.canonicalize() {
        Ok(file) => file,
        Err(_) => {
            let inspect = vm.val_inspect(args[0]);
            return Err(vm.error_argument(format!("Invalid filename. {}", inspect)));
        }
    };
    let mut file = match File::open(&filename) {
        Ok(file) => file,
        Err(_) => return Err(vm.error_internal(format!("Can not open file. {:?}", &filename))),
    };
    let mut contents = String::new();
    match file.read_to_string(&mut contents) {
        Ok(file) => file,
        Err(_) => return Err(vm.error_internal("Could not read the file.")),
    };
    Ok(Value::string(&vm.globals, contents))
}

/// IO.write(path, string)
fn write(vm: &mut VM, _: Value, args: &Args) -> VMResult {
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
    Ok(Value::fixnum(contents.len() as i64))
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn file() {
        let program = r#"
            File.write("file.txt","foo")
            assert("foo", File.read("file.txt"))
            File.write("file.txt","bar")
            assert("bar", File.read("file.txt"))
            assert("file.txt", File.basename("/usr/file.txt"))
            assert(".txt", File.extname("file.txt"))
        "#;
        assert_script(program);
    }
}
