use crate::*;
use arraystring::{typenum::U23, ArrayString};

pub type SmallString = ArrayString<U23>;

#[derive(Clone, PartialEq)]
pub enum RString {
    Str(String),
    SmallStr(SmallString),
    Bytes(Vec<u8>),
}

use std::fmt;
impl fmt::Debug for RString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn write(s: &str, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            for ch in s.chars() {
                match ch {
                    c @ '\'' | c @ '"' | c @ '\\' => {
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
        match self {
            RString::Str(s) => write(s, f),
            RString::SmallStr(s) => write(s, f),
            RString::Bytes(v) => write!(f, "{}", String::from_utf8_lossy(v)),
        }
    }
}

impl fmt::Display for RString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RString::Str(s) => write!(f, "{}", s),
            RString::SmallStr(s) => write!(f, "{}", s.as_str()),
            RString::Bytes(v) => write!(f, "{}", String::from_utf8_lossy(v)),
        }
    }
}

use std::borrow::Cow;
impl RString {
    /// Converts an object of Cow<str> or &str or String to a RString::Str.
    pub fn from<'a>(s: impl Into<Cow<'a, str>>) -> Self {
        let s = s.into();
        if s.len() <= SmallString::capacity() as usize {
            RString::SmallStr(SmallString::from_str_truncate(s))
        } else {
            RString::Str(s.into_owned())
        }
    }

    /// Converts a Vec<u8> to a RString.
    ///
    /// If a Vec<u8> is valid as UTF-8, converts to a RString::Str or ::SmallStr.
    pub fn from_bytes(b: Vec<u8>) -> Self {
        match std::str::from_utf8(&b) {
            Ok(s) => RString::from(s),
            Err(_) => RString::Bytes(b),
        }
    }

    pub fn append(&mut self, rhs: &RString) {
        use self::RString::*;
        use std::mem;
        *self = match *self {
            Str(ref mut lhs) => match rhs {
                Str(rhs) => {
                    *lhs += rhs;
                    return;
                }
                SmallStr(rhs) => {
                    *lhs += rhs.as_str();
                    return;
                }
                Bytes(rhs) => {
                    let mut bytes = mem::replace(lhs, String::new()).into_bytes();
                    bytes.extend_from_slice(rhs);
                    RString::from_bytes(bytes)
                }
            },
            SmallStr(ref mut lhs) => {
                let mut lhs = mem::replace(lhs, SmallString::new()).to_string();
                match rhs {
                    Str(rhs) => {
                        lhs += rhs;
                        Str(lhs)
                    }
                    SmallStr(rhs) => {
                        lhs += rhs.as_str();
                        RString::from(lhs)
                    }
                    Bytes(rhs) => {
                        let mut bytes = lhs.into_bytes();
                        bytes.extend_from_slice(rhs);
                        RString::from_bytes(bytes)
                    }
                }
            }
            Bytes(ref mut lhs) => match rhs {
                Str(rhs) => {
                    let mut bytes = mem::replace(lhs, Vec::new());
                    bytes.extend_from_slice(rhs.as_bytes());
                    RString::from_bytes(bytes)
                }
                SmallStr(rhs) => {
                    let mut bytes = mem::replace(lhs, Vec::new());
                    bytes.extend_from_slice(rhs.as_bytes());
                    RString::from_bytes(bytes)
                }
                Bytes(rhs) => {
                    lhs.extend_from_slice(rhs);
                    return;
                }
            },
        };
    }

    pub fn remove(&mut self, idx: usize) -> char {
        match self {
            RString::Str(s) => s.remove(idx),
            RString::SmallStr(s) => s.remove(idx as u8).unwrap(),
            RString::Bytes(v) => v.remove(idx) as char,
        }
    }
}

use std::cmp::Ordering;
use std::str::FromStr;
impl RString {
    pub fn as_string(&mut self) -> Result<&str, RubyError> {
        match self {
            RString::Str(s) => Ok(s),
            RString::SmallStr(s) => Ok(s.as_str()),
            RString::Bytes(bytes) => match std::str::from_utf8(bytes) {
                Ok(s) => {
                    //let mut_rstring = self as *const RString as *mut RString;
                    // Convert RString::Bytes => RString::Str in place.
                    *self = RString::from(s);
                    Ok(self.as_str())
                }
                Err(_) => Err(RubyError::argument("Invalid as UTF-8 string.")),
            },
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            RString::Str(s) => s,
            RString::SmallStr(s) => s.as_str(),
            RString::Bytes(_) => panic!(),
        }
    }

    /// Take reference of [u8] from RString.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            RString::Str(s) => s.as_bytes(),
            RString::SmallStr(s) => s.as_bytes(),
            RString::Bytes(b) => b,
        }
    }

    pub fn char_indices(&self) -> std::str::CharIndices<'_> {
        match self {
            RString::Str(s) => s.char_indices(),
            RString::SmallStr(s) => s.as_str().char_indices(),
            RString::Bytes(_) => panic!(),
        }
    }

    pub fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where
        R: std::ops::RangeBounds<usize>,
    {
        match self {
            RString::Str(s) => s.replace_range(range, replace_with),
            RString::SmallStr(s) => {
                let mut s = s.to_string();
                s.replace_range(range, replace_with);
                *self = RString::from(&s);
            }
            RString::Bytes(_) => panic!(),
        }
    }

    pub fn replacen<'a, P>(&'a mut self, pat: P, to: &str, count: usize) -> String
    where
        P: std::str::pattern::Pattern<'a>,
    {
        match self {
            RString::Str(s) => s.replacen(pat, to, count),
            RString::SmallStr(s) => s.as_mut_str().replacen(pat, to, count),
            RString::Bytes(_) => panic!(),
        }
    }

    /// Parse string as i64 or f64.
    pub fn parse<F: FromStr>(&self) -> Option<F> {
        match self {
            RString::Str(s) => FromStr::from_str(s).ok(),
            RString::SmallStr(s) => FromStr::from_str(s).ok(),
            RString::Bytes(bytes) => match std::str::from_utf8(bytes) {
                Ok(s) => FromStr::from_str(s).ok(),
                Err(_) => None,
            },
        }
    }

    pub fn to_s(&self) -> Cow<str> {
        match self {
            RString::Str(s) => Cow::from(s),
            RString::SmallStr(s) => Cow::from(s.as_str()),
            RString::Bytes(v) => String::from_utf8_lossy(v),
        }
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
        Some(RString::string_cmp(lhs, rhs))
    }

    pub fn string_cmp(lhs: &[u8], rhs: &[u8]) -> Ordering {
        if lhs.len() >= rhs.len() {
            for (i, rhs_v) in rhs.iter().enumerate() {
                match lhs[i].cmp(rhs_v) {
                    Ordering::Equal => {}
                    ord => return ord,
                }
            }
            if lhs.len() == rhs.len() {
                Ordering::Equal
            } else {
                Ordering::Greater
            }
        } else {
            for (i, lhs_v) in lhs.iter().enumerate() {
                match lhs_v.cmp(&rhs[i]) {
                    Ordering::Equal => {}
                    ord => return ord,
                }
            }
            Ordering::Less
        }
    }
}

impl std::hash::Hash for RString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            RString::Str(s) => s.hash(state),
            RString::SmallStr(s) => s.hash(state),
            RString::Bytes(b) => b.hash(state),
        };
    }
}
