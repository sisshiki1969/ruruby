use crate::*;
use std::collections::HashSet;
use std::fs;
use std::path::*;

pub fn init(_globals: &mut Globals) -> Value {
    let class = ClassRef::from_str("Dir", BuiltinClass::object());
    let mut class_val = Value::class(class);
    class_val.add_builtin_class_method("home", home);
    class_val.add_builtin_class_method("pwd", pwd);
    class_val.add_builtin_class_method("glob", glob);
    class_val.add_builtin_class_method("[]", glob);
    class_val
}

// Singleton methods

fn home(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let home_dir = dirs::home_dir()
        .unwrap_or(PathBuf::new())
        .to_string_lossy()
        .to_string();
    Ok(Value::string(home_dir))
}

fn pwd(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let cur_dir = std::env::current_dir()
        .unwrap_or(PathBuf::new())
        .to_string_lossy()
        .to_string();
    Ok(Value::string(cur_dir))
}

fn glob(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let mut pat_val = args[0];
    let mut pattern = pat_val.expect_string(vm, "1st arg")?.chars().peekable();
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
        return Ok(Value::array_from(vec![]));
    }
    //eprintln!("{:?}", glob);
    let mut matches = HashSet::new();
    match traverse_dir(&fullpath, &path, &glob, 0, &mut matches) {
        Ok(_) => {}
        Err(err) => return Err(vm.error_internal(format!("{:?}", err))),
    };
    Ok(Value::array_from(matches.into_iter().collect()))
}

fn traverse_dir(
    full_path: &PathBuf,
    path: &PathBuf,
    glob: &Vec<Option<regex::Regex>>,
    level: usize,
    matches: &mut HashSet<Value>,
) -> std::io::Result<()> {
    #[derive(Debug, PartialEq)]
    enum MatchState {
        Match,
        WildMatch,
        NextMatch,
    }

    if level == glob.len() {
        let path = path.to_string_lossy().to_string();
        matches.insert(Value::string(path));
        return Ok(());
    }
    assert!(level < glob.len());
    for entry in fs::read_dir(&full_path)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_cow = name.to_string_lossy();
        let state = match &glob[level] {
            Some(re) => {
                if re.find(name_cow.as_ref()).is_none() {
                    continue;
                } else {
                    MatchState::Match
                }
            }
            None => match glob.get(level + 1) {
                Some(Some(re)) if re.find(name_cow.as_ref()).is_some() => MatchState::NextMatch,
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
            let path = path.to_string_lossy().to_string();
            matches.insert(Value::string(path));
        }
    }
    Ok(())
}
