use crate::vm::*;
use crate::*;

#[derive(Clone, PartialEq)]
pub enum RString {
    Str(String),
    Bytes(Vec<u8>),
}

use std::fmt;
impl fmt::Debug for RString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RString::Str(s) => {
                for ch in s.chars() {
                    match ch {
                        c @ '\'' | c @ '"' | c @ '?' | c @ '\\' => {
                            write!(f, "\\{}", c)?;
                        }
                        '\x07' => write!(f, "\\a")?,
                        '\x08' => write!(f, "\\b")?,
                        '\x1b' => write!(f, "\\e")?,
                        '\x0c' => write!(f, "\\f")?,
                        '\x0a' => write!(f, "\\n")?,
                        '\x0d' => write!(f, "\\r")?,
                        '\x09' => write!(f, "\\t")?,
                        '\x0b' => write!(f, "\\v")?,
                        c => write!(f, "{}", c)?,
                    };
                }
                Ok(())
            }
            RString::Bytes(v) => write!(f, "{}", String::from_utf8_lossy(v)),
        }
    }
}

impl fmt::Display for RString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RString::Str(s) => write!(f, "{}", s),
            RString::Bytes(v) => write!(f, "{}", String::from_utf8_lossy(v)),
        }
    }
}

use std::cmp::Ordering;
use std::str::FromStr;
impl RString {
    /// Try to take reference of String from RString.
    /// If byte sequence is invalid as UTF-8, return Err.
    /// When valid, convert the byte sequence to UTF-8 string.
    pub fn as_string(&mut self, vm: &VM) -> Result<&String, RubyError> {
        match self {
            RString::Str(s) => Ok(s),
            RString::Bytes(bytes) => match String::from_utf8(bytes.clone()) {
                Ok(s) => {
                    //let mut_rstring = self as *const RString as *mut RString;
                    // Convert RString::Bytes => RString::Str in place.
                    *self = RString::Str(s);
                    let s = match self {
                        RString::Str(s) => s,
                        RString::Bytes(_) => unreachable!(),
                    };
                    Ok(s)
                }
                Err(_) => Err(vm.error_argument("Invalid as UTF-8 string.")),
            },
        }
    }

    /// Take reference of [u8] from RString.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            RString::Str(s) => s.as_bytes(),
            RString::Bytes(b) => b,
        }
    }

    /// Parse string as i64 or f64.
    pub fn parse<F: FromStr>(&self) -> Option<F> {
        match self {
            RString::Str(s) => FromStr::from_str(s).ok(),
            RString::Bytes(bytes) => match String::from_utf8(bytes.clone()) {
                Ok(s) => FromStr::from_str(&s).ok(),
                Err(_) => None,
            },
        }
    }

    pub fn to_s(&self) -> String {
        format!("{}", self)
    }

    pub fn inspect(&self) -> String {
        format!(r#""{:?}""#, self)
    }

    pub fn cmp(&self, other: Value) -> Option<Ordering> {
        let lhs = self.as_bytes();
        let rhs = match other.as_bytes() {
            Some(s) => s,
            None => return None,
        };
        if lhs.len() >= rhs.len() {
            for (i, rhs_v) in rhs.iter().enumerate() {
                match lhs[i].cmp(rhs_v) {
                    Ordering::Equal => {}
                    ord => return Some(ord),
                }
            }
            if lhs.len() == rhs.len() {
                Some(Ordering::Equal)
            } else {
                Some(Ordering::Greater)
            }
        } else {
            for (i, lhs_v) in lhs.iter().enumerate() {
                match lhs_v.cmp(&rhs[i]) {
                    Ordering::Equal => {}
                    ord => return Some(ord),
                }
            }
            Some(Ordering::Less)
        }
    }
}

impl std::hash::Hash for RString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            RString::Str(s) => s.hash(state),
            RString::Bytes(b) => b.hash(state),
        };
    }
}
