use crate::error::RubyError;
use crate::vm::*;
use fancy_regex::{Captures, Error, Match, Regex};
//#[macro_use]
use crate::*;

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

    pub fn from_string(reg_str: &str) -> Result<Self, Error> {
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

fn regexp_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let string = vm.expect_string(&args[0], "1st arg")?;
    let val = vm.create_regexp_from_string(string)?;
    Ok(val)
}

fn regexp_escape(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let string = vm.expect_string(&args[0], "1st arg")?;
    let res = regex::escape(string);
    let regexp = Value::string(&vm.globals, res);
    Ok(regexp)
}

// Instance methods

// Utility methods

impl Regexp {
    fn get_captures(vm: &mut VM, captures: &Captures, given: &str) {
        let id1 = vm.globals.get_ident_id("$&");
        let id2 = vm.globals.get_ident_id("$'");
        match captures.get(0) {
            Some(m) => {
                let val = Value::string(&vm.globals, given[m.start()..m.end()].to_string());
                vm.set_global_var(id1, val);
                let val = Value::string(&vm.globals, given[m.end()..].to_string());
                vm.set_global_var(id2, val);
            }
            None => {
                vm.set_global_var(id1, Value::nil());
                vm.set_global_var(id2, Value::nil());
            }
        };

        for i in 1..captures.len() {
            match captures.get(i) {
                Some(m) => Regexp::set_special_global(vm, i, given, m.start(), m.end()),
                None => Regexp::set_special_global_nil(vm, i),
            };
        }
    }

    fn set_special_global(vm: &mut VM, i: usize, given: &str, start: usize, end: usize) {
        let id = vm.globals.get_ident_id(format!("${}", i));
        let val = Value::string(&vm.globals, given[start..end].to_string());
        //eprintln!("${}: {}", i, given[start..end].to_string());
        vm.set_global_var(id, val);
    }

    fn set_special_global_nil(vm: &mut VM, i: usize) {
        let id = vm.globals.get_ident_id(format!("${}", i));
        vm.set_global_var(id, Value::nil());
    }

    /// Replaces the leftmost-first match with `replace`.
    pub fn replace_one(
        vm: &mut VM,
        re_val: Value,
        given: &str,
        replace: &str,
    ) -> Result<String, RubyError> {
        fn replace_(
            vm: &mut VM,
            re: &Regexp,
            given: &str,
            replace: &str,
        ) -> Result<String, RubyError> {
            match re.captures(given) {
                Ok(None) => Ok(given.to_string()),
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
                    Ok(res)
                }
                Err(err) => return Err(vm.error_internal(format!("Capture failed. {:?}", err))),
            }
        }

        if let Some(s) = re_val.as_string() {
            let re = vm.regexp_from_string(&s)?;
            return replace_(vm, &re, given, replace);
        } else if let Some(re) = re_val.as_regexp() {
            return replace_(vm, &re.regexp, given, replace);
        } else {
            return Err(vm.error_argument("1st arg must be RegExp or String."));
        };
    }

    pub fn replace_one_block(
        vm: &mut VM,
        re_val: Value,
        given: &str,
        block: MethodRef,
    ) -> Result<(String, bool), RubyError> {
        fn replace_(
            vm: &mut VM,
            re: &Regexp,
            given: &str,
            block: MethodRef,
        ) -> Result<(String, bool), RubyError> {
            let (start, end, matched_str) = match re.captures_from_pos(given, 0) {
                Ok(None) => return Ok((given.to_string(), false)),
                Ok(Some(captures)) => {
                    let m = captures.get(0).unwrap();
                    Regexp::get_captures(vm, &captures, given);
                    (m.start(), m.end(), m.as_str())
                }
                Err(err) => return Err(vm.error_internal(format!("Capture failed. {:?}", err))),
            };

            let mut res = given.to_string();
            let matched = Value::string(&vm.globals, matched_str.to_string());
            let result = vm.eval_block(block, &Args::new1(matched))?;
            let s = vm.val_to_s(result);
            res.replace_range(start..end, &s);
            Ok((res, true))
        }

        if let Some(s) = re_val.as_string() {
            let re = vm.regexp_from_string(&s)?;
            return replace_(vm, &re, given, block);
        } else if let Some(re) = re_val.as_regexp() {
            return replace_(vm, &re.regexp, given, block);
        } else {
            return Err(vm.error_argument("1st arg must be RegExp or String."));
        };
    }

    /// Replaces all non-overlapping matches in `given` string with `replace`.
    pub fn replace_all(
        vm: &mut VM,
        re_val: Value,
        given: &str,
        replace: &str,
    ) -> Result<(String, bool), RubyError> {
        fn replace_(
            vm: &mut VM,
            re: &Regexp,
            given: &str,
            replace: &str,
        ) -> Result<(String, bool), RubyError> {
            let mut range = vec![];
            let mut i = 0;
            loop {
                if i >= given.len() {
                    break;
                }
                match re.captures_from_pos(given, i) {
                    Ok(None) => break,
                    Ok(Some(captures)) => {
                        let m = captures.get(0).unwrap();
                        // the length of matched string can be 0.
                        // this is neccesary to avoid infinite loop.
                        i = if m.end() == m.start() {
                            m.end() + 1
                        } else {
                            m.end()
                        };
                        range.push((m.start(), m.end()));
                        //eprintln!("{} {} [{:?}]", m.start(), m.end(), m.as_str());
                        Regexp::get_captures(vm, &captures, given);
                    }
                    Err(err) => return Err(vm.error_internal(format!("Capture failed. {:?}", err))),
                };
            }
            let mut res = given.to_string();
            for (start, end) in range.iter().rev() {
                res.replace_range(start..end, replace);
            }
            Ok((res, range.len() != 0))
        }

        if let Some(s) = re_val.as_string() {
            let re = vm.regexp_from_string(&s)?;
            return replace_(vm, &re, given, replace);
        } else if let Some(re) = re_val.as_regexp() {
            return replace_(vm, &re.regexp, given, replace);
        } else {
            return Err(vm.error_argument("1st arg must be RegExp or String."));
        };
    }

    /// Replaces all non-overlapping matches in `given` string with `replace`.
    pub fn replace_all_block(
        vm: &mut VM,
        re_val: Value,
        given: &str,
        block: MethodRef,
    ) -> Result<(String, bool), RubyError> {
        fn replace_(
            vm: &mut VM,
            re: &Regexp,
            given: &str,
            block: MethodRef,
        ) -> Result<(String, bool), RubyError> {
            let mut range = vec![];
            let mut i = 0;
            loop {
                let (start, end, matched_str) = match re.captures_from_pos(given, i) {
                    Ok(None) => break,
                    Ok(Some(captures)) => {
                        let m = captures.get(0).unwrap();
                        i = m.end();
                        Regexp::get_captures(vm, &captures, given);
                        (m.start(), m.end(), m.as_str())
                    }
                    Err(err) => return Err(vm.error_internal(format!("Capture failed. {:?}", err))),
                };
                let matched = Value::string(&vm.globals, matched_str.to_string());
                let result = vm.eval_block(block, &Args::new1(matched))?;
                let replace = vm.val_to_s(result);
                range.push((start, end, replace));
            }

            let mut res = given.to_string();
            for (start, end, replace) in range.iter().rev() {
                res.replace_range(start..end, replace);
            }
            Ok((res, range.len() != 0))
        }

        if let Some(s) = re_val.as_string() {
            let re = vm.regexp_from_string(&s)?;
            return replace_(vm, &re, given, block);
        } else if let Some(re) = re_val.as_regexp() {
            return replace_(vm, &re.regexp, given, block);
        } else {
            return Err(vm.error_argument("1st arg must be RegExp or String."));
        };
    }

    pub fn find_one<'a>(
        vm: &mut VM,
        re: &Regexp,
        given: &'a str,
    ) -> Result<Option<Match<'a>>, RubyError> {
        match re.captures(given) {
            Ok(None) => Ok(None),
            Ok(Some(captures)) => {
                Regexp::get_captures(vm, &captures, given);
                Ok(captures.get(0))
            }
            Err(err) => Err(vm.error_internal(format!("Capture failed. {:?}", err))),
        }
    }

    pub fn find_all(vm: &mut VM, re: &Regexp, given: &str) -> Result<Vec<Value>, RubyError> {
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
                            let val =
                                Value::string(&vm.globals, given[m.start()..m.end()].to_string());
                            ary.push(val);
                        }
                        len => {
                            let mut vec = vec![];
                            for i in 1..len {
                                match captures.get(i) {
                                    Some(m) => {
                                        let s = given[m.start()..m.end()].to_string();
                                        vec.push(Value::string(&vm.globals, s));
                                    }
                                    None => vec.push(Value::nil()),
                                }
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

#[cfg(test)]
mod test {
    use crate::test::*;

    #[test]
    fn regexp1() {
        let program = r#"
    assert "abc!!g", "abcdefg".gsub(/def/, "!!")
    assert "2.5".gsub(".", ","), "2,5"
    assert true, /(aa).*(bb)/ === "andaadefbbje"
    assert "aadefbb", $&
    assert "aa", $1
    assert "bb", $2
    "#;
        assert_script(program);
    }
}
