use crate::*;
use fxhash::FxHashSet;
use std::fs;
use std::path::*;

pub fn init() -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Dir", class);
    class.add_builtin_class_method("home", home);
    class.add_builtin_class_method("pwd", pwd);
    class.add_builtin_class_method("glob", glob);
    class.add_builtin_class_method("[]", glob);
    class.add_builtin_class_method("exist?", exist);
    class.into()
}

// Singleton methods

fn home(_: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let home_dir = dirs::home_dir().unwrap_or(PathBuf::new());
    Ok(Value::string(conv_pathbuf(&home_dir)))
}

fn pwd(_: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let cur_dir = std::env::current_dir().unwrap_or(PathBuf::new());
    Ok(Value::string(conv_pathbuf(&cur_dir)))
}

fn exist(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let res = match super::file::string_to_canonicalized_path(vm, args[0], "1st arg") {
        Ok(path) => path,
        Err(_) => return Ok(Value::false_val()),
    };
    Ok(Value::bool(res.is_dir()))
}

fn glob(_: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut pat_val = args[0];
    let mut pattern = pat_val.expect_string("1st arg")?.chars().peekable();
    let mut glob: Vec<String> = vec![];
    let mut charbuf = vec!['^'];
    let (fullpath, path) = if pattern.peek() == Some(&'/') {
        pattern.next();
        (PathBuf::from("/"), PathBuf::from("/"))
    } else {
        (std::env::current_dir().unwrap(), PathBuf::new())
    };
    loop {
        match pattern.next() {
            Some(ch) => match ch {
                '?' => charbuf.push('.'),
                '*' => {
                    if charbuf.len() == 1 {
                        if pattern.peek() == Some(&'*') {
                            pattern.next();
                            if pattern.peek() == Some(&'/') {
                                pattern.next();
                                glob.push("*".to_string());
                                charbuf = vec!['^'];
                                continue;
                            }
                        }
                        charbuf.append(&mut r#"(?:[^\.].*)?"#.chars().collect());
                    } else {
                        charbuf.push('.');
                        charbuf.push('*');
                    }
                }
                '.' => {
                    charbuf.push('\\');
                    charbuf.push('.');
                }
                '/' => {
                    charbuf.push('$');
                    glob.push(charbuf.iter().collect());
                    charbuf = vec!['^'];
                }
                _ => charbuf.push(ch),
            },
            None => {
                if !charbuf.is_empty() {
                    charbuf.push('$');
                    glob.push(charbuf.iter().collect());
                }
                break;
            }
        }
    }
    let glob: Vec<_> = glob
        .iter()
        .map(|s| {
            if s == "*" {
                None
            } else {
                Some(regex::Regex::new(s).unwrap())
            }
        })
        .collect();
    if glob.is_empty() {
        return Ok(Value::array_empty());
    }
    //eprintln!("{:?}", glob);
    let mut matches = FxHashSet::default();
    match traverse_dir(&fullpath, &path, &glob, 0, &mut matches) {
        Ok(_) => {}
        Err(err) => return Err(RubyError::internal(format!("{:?}", err))),
    };
    Ok(Value::array_from(
        matches.into_iter().map(|s| Value::string(s)).collect(),
    ))
}

fn traverse_dir(
    full_path: &PathBuf,
    path: &PathBuf,
    glob: &Vec<Option<regex::Regex>>,
    level: usize,
    matches: &mut FxHashSet<String>,
) -> std::io::Result<()> {
    #[derive(Debug, PartialEq)]
    enum MatchState {
        Match,
        WildMatch,
        NextMatch,
    }

    if level == glob.len() {
        matches.insert(conv_pathbuf(path));
        return Ok(());
    }
    assert!(level < glob.len());
    for entry in fs::read_dir(&full_path)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_cow = name.to_string_lossy();
        let state = match &glob[level] {
            Some(re) => {
                if re.find(&name_cow).is_none() {
                    continue;
                } else {
                    MatchState::Match
                }
            }
            None => match glob.get(level + 1) {
                Some(Some(re)) if re.find(&name_cow).is_some() => MatchState::NextMatch,
                _ => MatchState::WildMatch,
            },
        };
        if entry.file_type()?.is_dir() {
            let mut full_path = full_path.clone();
            full_path.push(name_cow.as_ref());
            let mut path = path.clone();
            path.push(name_cow.as_ref());
            match state {
                MatchState::Match => traverse_dir(&full_path, &path, glob, level + 1, matches)?,
                MatchState::NextMatch => {
                    traverse_dir(&full_path, &path, glob, level + 2, matches)?;
                    if !name_cow.as_ref().starts_with('.') {
                        traverse_dir(&full_path, &path, glob, level, matches)?;
                    }
                }
                MatchState::WildMatch => {
                    if !name_cow.as_ref().starts_with('.') {
                        traverse_dir(&full_path, &path, glob, level + 1, matches)?;
                        traverse_dir(&full_path, &path, glob, level, matches)?;
                    }
                }
            };
        } else if level == glob.len() - 1
            || state == MatchState::NextMatch && level + 1 == glob.len() - 1
        {
            let mut path = path.clone();
            path.push(name_cow.as_ref());
            matches.insert(conv_pathbuf(&path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::tests::*;
    #[test]
    fn dir_test() {
        let program = r##"
        assert ENV["HOME"], Dir.home
        #assert ENV["PWD"], Dir.pwd  this fails in GitHub Actions 2021.2
        assert ["src/builtin/enumerator.rs"], Dir["**/en*?.rs"]
        assert [
            "src/alloc.rs","src/arith.rs","src/builtin/array.rs",
            "src/coroutine/asm_windows_x64.rs",
            "src/coroutine/asm_x64.rs",
            "src/coroutine/asm_arm64.rs",
            "src/parse/parser/arguments.rs",
            "src/value/array.rs",
            "src/vm/args.rs"
        ].sort, Dir["src/**/a*s"].sort
        assert true, Dir.exist?("src")
        assert false, Dir.exist?("srd")
        assert false, Dir.exist?("Cargo.toml")
    "##;
        assert_script(program);
    }
}
