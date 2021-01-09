use crate::error::RubyError;
use crate::vm::*;
use fancy_regex::{Captures, Error, Match, Regex};
//#[macro_use]
use crate::*;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct RegexpInfo(Rc<Regex>);

impl RegexpInfo {
    pub fn from_escaped(globals: &mut Globals, escaped_str: &str) -> Result<Self, Error> {
        let string = regex::escape(escaped_str);
        RegexpInfo::from_string(globals, &string)
    }

    pub fn from_string(globals: &mut Globals, reg_str: &str) -> Result<Self, Error> {
        match globals.regexp_cache.get(reg_str) {
            Some(re) => Ok(RegexpInfo(re.clone())),
            None => {
                //eprintln!("new: {}", reg_str);
                let regex = Rc::new(Regex::new(reg_str)?);
                globals
                    .regexp_cache
                    .insert(reg_str.to_string(), regex.clone());
                Ok(RegexpInfo(regex))
            }
        }
    }
}

impl RegexpInfo {
    /// Replace all matches for `self` in `given` string with `replace`.
    ///
    /// ### return
    /// (replaced:String, is_replaced?:bool)
    pub fn replace_repeat(
        &self,
        vm: &mut VM,
        given: &str,
        replace: &str,
    ) -> Result<(String, bool), RubyError> {
        let mut range = vec![];
        let mut i = 0;
        loop {
            if i >= given.len() {
                break;
            }
            match self.captures_from_pos(given, i) {
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
                    RegexpInfo::get_captures(vm, &captures, given);
                }
                Err(err) => return Err(RubyError::internal(format!("Capture failed. {:?}", err))),
            };
        }
        let mut res = given.to_string();
        for (start, end) in range.iter().rev() {
            res.replace_range(start..end, replace);
        }
        Ok((res, range.len() != 0))
    }

    /// Replaces the leftmost-first match for `self` in `given` string with `replace`.
    ///
    /// ### return
    /// replaced:String
    pub fn replace_once<'a>(
        &'a self,
        vm: &mut VM,
        given: &'a str,
        replace: &str,
    ) -> Result<(String, Option<Captures>), RubyError> {
        match self.captures(given) {
            Ok(None) => Ok((given.to_string(), None)),
            Ok(Some(captures)) => {
                let mut res = given.to_string();
                let m = captures.get(0).unwrap();
                RegexpInfo::get_captures(vm, &captures, given);
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
                Ok((res, Some(captures)))
            }
            Err(err) => return Err(RubyError::internal(format!("Capture failed. {:?}", err))),
        }
    }
}

impl PartialEq for RegexpInfo {
    fn eq(&self, other: &Self) -> bool {
        if Rc::ptr_eq(&self.0, &other.0) {
            return true;
        }
        self.as_str() == other.as_str()
    }
}

impl std::ops::Deref for RegexpInfo {
    type Target = Regex;
    fn deref(&self) -> &Regex {
        &self.0
    }
}

pub fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under(globals.builtins.object);
    class.add_builtin_class_method("new", regexp_new);
    class.add_builtin_class_method("compile", regexp_new);
    class.add_builtin_class_method("escape", regexp_escape);
    class.add_builtin_class_method("quote", regexp_escape);
    class.add_builtin_method_by_str("=~", regexp_match);
    class.into()
}

// Class methods

fn regexp_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut arg0 = args[0];
    let string = arg0.expect_string("1st arg")?;
    let val = Value::regexp_from(vm, string)?;
    Ok(val)
}

fn regexp_escape(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut arg0 = args[0];
    let string = arg0.expect_string("1st arg")?;
    let regexp = Value::string(regex::escape(string));
    Ok(regexp)
}

fn regexp_match(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut args0 = args[0];
    let regex = self_val.as_regexp().unwrap();
    let given = args0.expect_string("1st Arg")?;
    let res = match RegexpInfo::find_one(vm, &regex, given).unwrap() {
        Some(mat) => Value::integer(mat.start() as i64),
        None => Value::nil(),
    };
    return Ok(res);
}

// Instance methods

// Utility methods

impl RegexpInfo {
    fn get_captures(vm: &mut VM, captures: &Captures, given: &str) {
        let id1 = IdentId::get_id("$&");
        let id2 = IdentId::get_id("$'");
        match captures.get(0) {
            Some(m) => {
                let val = Value::string(&given[m.start()..m.end()]);
                vm.set_global_var(id1, val);
                let val = Value::string(&given[m.end()..]);
                vm.set_global_var(id2, val);
            }
            None => {
                vm.set_global_var(id1, Value::nil());
                vm.set_global_var(id2, Value::nil());
            }
        };

        for i in 1..captures.len() {
            match captures.get(i) {
                Some(m) => Self::set_special_global(vm, i, given, m.start(), m.end()),
                None => Self::set_special_global_nil(vm, i),
            };
        }
    }

    fn set_special_global(vm: &mut VM, i: usize, given: &str, start: usize, end: usize) {
        let id = IdentId::get_id(&format!("${}", i));
        let val = Value::string(&given[start..end]);
        vm.set_global_var(id, val);
    }

    fn set_special_global_nil(vm: &mut VM, i: usize) {
        let id = IdentId::get_id(&format!("${}", i));
        vm.set_global_var(id, Value::nil());
    }

    /// Replaces the leftmost-first match with `replace`.
    pub fn replace_one(
        vm: &mut VM,
        re_val: Value,
        given: &str,
        replace: &str,
    ) -> Result<String, RubyError> {
        if let Some(s) = re_val.as_string() {
            let re = vm.regexp_from_escaped_string(&s)?;
            return re.replace_once(vm, given, replace).map(|x| x.0);
        } else if let Some(re) = re_val.as_regexp() {
            return re.replace_once(vm, given, replace).map(|x| x.0);
        } else {
            return Err(RubyError::argument("1st arg must be RegExp or String."));
        };
    }

    pub fn replace_one_block(
        vm: &mut VM,
        re_val: Value,
        given: &str,
        block: &Block,
    ) -> Result<(String, bool), RubyError> {
        fn replace_(
            vm: &mut VM,
            re: &RegexpInfo,
            given: &str,
            block: &Block,
        ) -> Result<(String, bool), RubyError> {
            let (start, end, matched_str) = match re.captures_from_pos(given, 0) {
                Ok(None) => return Ok((given.to_string(), false)),
                Ok(Some(captures)) => {
                    let m = captures.get(0).unwrap();
                    RegexpInfo::get_captures(vm, &captures, given);
                    (m.start(), m.end(), m.as_str())
                }
                Err(err) => return Err(RubyError::internal(format!("Capture failed. {:?}", err))),
            };

            let mut res = given.to_string();
            let matched = Value::string(matched_str);
            let result = vm.eval_block(block, &Args::new1(matched))?;
            let s = result.val_to_s(vm)?;
            res.replace_range(start..end, &s);
            Ok((res, true))
        }

        if let Some(s) = re_val.as_string() {
            let re = vm.regexp_from_escaped_string(&s)?;
            return replace_(vm, &re, given, block);
        } else if let Some(re) = re_val.as_regexp() {
            return replace_(vm, &re, given, block);
        } else {
            return Err(RubyError::argument("1st arg must be RegExp or String."));
        };
    }

    /// Replaces all non-overlapping matches in `given` string with `replace`.
    pub fn replace_all(
        vm: &mut VM,
        regexp: Value,
        given: &str,
        replace: &str,
    ) -> Result<(String, bool), RubyError> {
        if let Some(s) = regexp.as_string() {
            let re = vm.regexp_from_escaped_string(&s)?;
            return re.replace_repeat(vm, given, replace);
        } else if let Some(re) = regexp.as_regexp() {
            return re.replace_repeat(vm, given, replace);
        } else {
            return Err(RubyError::argument("1st arg must be RegExp or String."));
        };
    }

    /// Replaces all non-overlapping matches in `given` string with `replace`.
    pub fn replace_all_block(
        vm: &mut VM,
        re_val: Value,
        given: &str,
        block: &Block,
    ) -> Result<(String, bool), RubyError> {
        fn replace_(
            vm: &mut VM,
            re: &RegexpInfo,
            given: &str,
            block: &Block,
        ) -> Result<(String, bool), RubyError> {
            let mut range = vec![];
            let mut i = 0;
            loop {
                let (start, end, matched_str) = match re.captures_from_pos(given, i) {
                    Ok(None) => break,
                    Ok(Some(captures)) => {
                        let m = captures.get(0).unwrap();
                        i = m.end();
                        RegexpInfo::get_captures(vm, &captures, given);
                        (m.start(), m.end(), m.as_str())
                    }
                    Err(err) => {
                        return Err(RubyError::internal(format!("Capture failed. {:?}", err)))
                    }
                };
                let matched = Value::string(matched_str);
                let result = vm.eval_block(block, &Args::new1(matched))?;
                let replace = result.val_to_s(vm)?.into_owned();
                range.push((start, end, replace));
            }

            let mut res = given.to_string();
            for (start, end, replace) in range.iter().rev() {
                res.replace_range(start..end, replace);
            }
            Ok((res, range.len() != 0))
        }

        if let Some(s) = re_val.as_string() {
            let re = vm.regexp_from_escaped_string(&s)?;
            return replace_(vm, &re, given, block);
        } else if let Some(re) = re_val.as_regexp() {
            return replace_(vm, &re, given, block);
        } else {
            return Err(RubyError::argument("1st arg must be RegExp or String."));
        };
    }

    pub fn match_one<'a>(
        vm: &mut VM,
        re: &Regex,
        given: &'a str,
        pos: usize,
    ) -> Result<Value, RubyError> {
        let pos = match given.char_indices().nth(pos) {
            Some((pos, _)) => pos,
            None => return Ok(Value::nil()),
        };
        match re.captures_from_pos(given, pos) {
            Ok(None) => Ok(Value::nil()),
            Ok(Some(captures)) => {
                RegexpInfo::get_captures(vm, &captures, given);
                let mut v = vec![];
                for i in 0..captures.len() {
                    v.push(Value::string(captures.get(i).unwrap().as_str()));
                }
                Ok(Value::array_from(v))
            }
            Err(err) => Err(RubyError::internal(format!("Capture failed. {:?}", err))),
        }
    }

    pub fn match_one_block<'a>(
        vm: &mut VM,
        re: &Regex,
        given: &'a str,
        block: &Block,
        pos: usize,
    ) -> Result<Value, RubyError> {
        let pos = match given.char_indices().nth(pos) {
            Some((pos, _)) => pos,
            None => return Ok(Value::nil()),
        };
        match re.captures_from_pos(given, pos) {
            Ok(None) => Ok(Value::nil()),
            Ok(Some(captures)) => {
                RegexpInfo::get_captures(vm, &captures, given);
                let matched = Value::string(captures.get(0).unwrap().as_str());
                vm.eval_block(block, &Args::new1(matched))
            }
            Err(err) => Err(RubyError::internal(format!("Capture failed. {:?}", err))),
        }
    }

    /// Find the leftmost-first match for `given`.
    /// Returns Matchs.
    pub fn find_one<'a>(
        vm: &mut VM,
        re: &Regex,
        given: &'a str,
    ) -> Result<Option<Match<'a>>, RubyError> {
        match re.captures(given) {
            Ok(None) => Ok(None),
            Ok(Some(captures)) => {
                RegexpInfo::get_captures(vm, &captures, given);
                Ok(captures.get(0))
            }
            Err(err) => Err(RubyError::internal(format!("Capture failed. {:?}", err))),
        }
    }

    pub fn find_all(vm: &mut VM, re: &Regex, given: &str) -> Result<Vec<Value>, RubyError> {
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
                            let val = Value::string(&given[m.start()..m.end()]);
                            ary.push(val);
                        }
                        len => {
                            let mut vec = vec![];
                            for i in 1..len {
                                match captures.get(i) {
                                    Some(m) => {
                                        vec.push(Value::string(&given[m.start()..m.end()]));
                                    }
                                    None => vec.push(Value::nil()),
                                }
                            }
                            let val = Value::array_from(vec);
                            ary.push(val);
                        }
                    }
                    last_captures = Some(captures);
                }
                Err(err) => return Err(RubyError::internal(format!("Capture failed. {:?}", err))),
            };
        }
        match last_captures {
            Some(c) => RegexpInfo::get_captures(vm, &c, given),
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
        assert 4, "The cat sat in the hat" =~ /[csh](..) [csh]\1 in/
        assert "x-xBBGZbbBBBVZc", "xbbgz-xbbbvzbbc".gsub(/(b+.z)(..)/) { $2 + $1.upcase }
    "#;
        assert_script(program);
    }

    #[test]
    fn regexp2() {
        let program = r#"
        assert 3, "aaazzz" =~ /\172+/
        assert 0, /foo/ =~ "foo"  # => 0
        assert 1, /foo/ =~ "afoo" # => 1
        assert nil, /foo/ =~ "bar"  # => nil
        "#;
        assert_script(program);
    }

    #[test]
    fn regexp_error() {
        assert_error(r#"/+/"#);
        assert_error(r#"Regexp.new("+")"#);
    }
}
