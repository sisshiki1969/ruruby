use crate::error::RubyError;
use crate::vm::*;
use fancy_regex::{Captures, Error, Match, Regex};

#[derive(Debug)]
pub struct RegexpInfo {
    pub regexp: Regexp,
}

impl RegexpInfo {
    pub fn new(regexp: Regex) -> Self {
        RegexpInfo {
            regexp: Regexp(regexp),
        }
    }
}

pub type RegexpRef = Ref<RegexpInfo>;

impl RegexpRef {
    pub fn from(reg: Regex) -> Self {
        RegexpRef::new(RegexpInfo::new(reg))
    }

    pub fn from_string(reg_str: &String) -> Result<Self, Error> {
        let regex = Regex::new(reg_str)?;
        Ok(RegexpRef::new(RegexpInfo::new(regex)))
    }
}

#[derive(Debug)]
pub struct Regexp(Regex);

impl std::ops::Deref for Regexp {
    type Target = Regex;
    fn deref(&self) -> &Regex {
        &self.0
    }
}

impl Regexp {
    pub fn new(re: Regex) -> Self {
        Regexp(re)
    }
}

pub fn init_regexp(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Regexp");
    let classref = ClassRef::from(id, globals.builtins.object);
    let regexp = Value::class(globals, classref);
    globals.add_builtin_class_method(regexp, "new", regexp_new);
    globals.add_builtin_class_method(regexp, "compile", regexp_new);
    globals.add_builtin_class_method(regexp, "escape", regexp_escape);
    globals.add_builtin_class_method(regexp, "quote", regexp_escape);
    regexp
}

// Class methods

fn regexp_new(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let val = match args[0].as_string() {
        Some(string) => vm.create_regexp(string)?,
        None => return Err(vm.error_argument("Must be String")),
    };
    Ok(val)
}

fn regexp_escape(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let res = match args[0].as_string() {
        Some(s) => regex::escape(s),
        None => return Err(vm.error_argument("Must be String")),
    };
    let regexp = Value::string(res);
    Ok(regexp)
}

// Instance methods

// Utility methods

impl Regexp {
    fn get_captures(vm: &mut VM, captures: &Captures, given: &String) {
        for i in 1..captures.len() {
            match captures.get(i) {
                Some(m) => Regexp::set_special_global(vm, i, given, m.start(), m.end()),
                None => Regexp::set_special_global_nil(vm, i),
            };
        }
    }

    fn set_special_global(vm: &mut VM, i: usize, given: &String, start: usize, end: usize) {
        let id = vm.globals.get_ident_id(format!("${}", i));
        let val = Value::string(given[start..end].to_string());
        vm.set_global_var(id, val);
    }

    fn set_special_global_nil(vm: &mut VM, i: usize) {
        let id = vm.globals.get_ident_id(format!("${}", i));
        vm.set_global_var(id, Value::nil());
    }

    /// Replaces the leftmost-first match with `replace`.
    pub fn replace_one(
        vm: &mut VM,
        re: &Regexp,
        given: &String,
        replace: &String,
    ) -> Result<String, String> {
        let res = match re.captures(given) {
            Ok(None) => given.to_string(),
            Ok(Some(captures)) => {
                let mut res = given.to_string();
                let m = captures.get(0).unwrap();
                Regexp::get_captures(vm, &captures, given);
                let mut rep = "".to_string();
                let mut escape = false;
                for ch in replace.chars() {
                    if escape {
                        match ch {
                            '0'..='9' => {
                                let i = ch as usize - '0' as usize;
                                match captures.get(i) {
                                    Some(m) => rep += m.as_str(),
                                    None => {}
                                };
                            }
                            _ => rep.push(ch),
                        };
                        escape = false;
                    } else {
                        if ch != '\\' {
                            rep.push(ch);
                        } else {
                            escape = true;
                        };
                    }
                }
                res.replace_range(m.start()..m.end(), &rep);
                res
            }
            Err(err) => return Err(format!("{:?}", err)),
        };
        Ok(res)
    }

    /// Replaces all non-overlapping matches in `given` string with `replace`.
    pub fn replace_all(
        vm: &mut VM,
        re: &Regexp,
        given: &String,
        replace: &String,
    ) -> Result<String, String> {
        let mut range = vec![];
        let mut i = 0;
        let mut last_captures = None;
        loop {
            match re.captures_from_pos(given, i) {
                Ok(None) => break,
                Ok(Some(captures)) => {
                    let m = captures.get(0).unwrap();
                    i = m.end();
                    range.push((m.start(), m.end()));
                    last_captures = Some(captures);
                }
                Err(err) => return Err(format!("{:?}", err)),
            };
        }
        match last_captures {
            Some(c) => Regexp::get_captures(vm, &c, given),
            None => {}
        }
        let mut res = given.to_string();
        for (start, end) in range.iter().rev() {
            res.replace_range(start..end, replace);
        }
        Ok(res)
    }

    pub fn find_one<'a>(
        vm: &mut VM,
        re: &Regexp,
        given: &'a String,
    ) -> Result<Option<Match<'a>>, Error> {
        match re.captures(given) {
            Ok(None) => Ok(None),
            Ok(Some(captures)) => {
                Regexp::get_captures(vm, &captures, given);
                Ok(captures.get(0))
            }
            Err(err) => Err(err),
        }
    }

    pub fn find_all(vm: &mut VM, re: &Regexp, given: &String) -> Result<Vec<Value>, RubyError> {
        let mut ary = vec![];
        let mut idx = 0;
        let mut last_captures = None;
        loop {
            match re.captures_from_pos(given, idx) {
                Ok(None) => break,
                Ok(Some(captures)) => {
                    let m = captures.get(0).unwrap();
                    idx = m.end();
                    match captures.len() {
                        1 => {
                            let val = Value::string(given[m.start()..m.end()].to_string());
                            ary.push(val);
                        }
                        len => {
                            let mut vec = vec![];
                            for i in 1..len {
                                let m = captures.get(i).unwrap();
                                let s = given[m.start()..m.end()].to_string();
                                vec.push(Value::string(s));
                            }
                            let val = Value::array_from(&vm.globals, vec);
                            ary.push(val);
                        }
                    }
                    last_captures = Some(captures);
                }
                Err(err) => return Err(vm.error_internal(format!("Capture failed. {:?}", err))),
            };
        }
        match last_captures {
            Some(c) => Regexp::get_captures(vm, &c, given),
            None => {}
        }
        Ok(ary)
    }
}
