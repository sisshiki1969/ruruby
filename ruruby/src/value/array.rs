use crate::*;
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy)]
pub struct Array(Value);

impl std::cmp::PartialEq for Array {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl std::cmp::Eq for Array {}

impl std::ops::Deref for Array {
    type Target = ArrayInfo;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        match self.0.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::ARRAY => &*oref.array(),
                _ => unreachable!(),
            },
            None => unreachable!(),
        }
    }
}

impl std::ops::DerefMut for Array {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.0.as_mut_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::ARRAY => oref.array_mut(),
                _ => unreachable!(),
            },
            None => unreachable!(),
        }
    }
}

impl std::hash::Hash for Array {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl Into<Value> for Array {
    #[inline(always)]
    fn into(self) -> Value {
        self.0
    }
}

impl GC for Array {
    fn mark(&self, alloc: &mut Allocator) {
        self.get().mark(alloc);
    }
}

impl Array {
    #[inline(always)]
    pub(crate) fn new_unchecked(val: Value) -> Self {
        Array(val)
    }

    #[inline(always)]
    fn get(self) -> Value {
        self.0
    }

    #[inline(always)]
    pub(crate) fn id(self) -> u64 {
        self.0.id()
    }
}

const ARRAY_INLINE: usize = 4;

/// Ruby Array.
///
/// # Examples
///
/// ```
/// # use ruruby::*;
/// let mut a = ArrayInfo::new(vec![]);
/// a.push(Value::integer(0));
/// a.push(Value::integer(1));
/// a.push(Value::integer(2));
/// assert_eq!(&*a, &[Value::integer(0), Value::integer(1), Value::integer(2)]);
/// a.push(Value::integer(3));
/// assert_eq!(&*a, &[Value::integer(0),
///                   Value::integer(1),
///                   Value::integer(2),
///                   Value::integer(3)]);
/// a.push(Value::integer(4));
/// assert_eq!(&*a, &[Value::integer(0),
///                   Value::integer(1),
///                   Value::integer(2),
///                   Value::integer(3),
///                   Value::integer(4)]);
/// a.pop();
/// a.pop();
/// a.pop();
/// assert_eq!(&*a, &[Value::integer(0), Value::integer(1)]);
/// ```
#[derive(Debug, Clone)]
pub struct ArrayInfo {
    inner: SmallVec<[Value; ARRAY_INLINE]>,
}

impl GC for ArrayInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.iter().for_each(|v| v.mark(alloc));
    }
}

impl std::ops::Deref for ArrayInfo {
    type Target = [Value];
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl std::ops::DerefMut for ArrayInfo {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl ArrayInfo {
    #[inline(always)]
    pub fn new(vec: Vec<Value>) -> Self {
        Self {
            inner: SmallVec::from_vec(vec),
        }
    }

    #[inline(always)]
    pub fn new_from_slice(slice: &[Value]) -> Self {
        Self {
            inner: SmallVec::from_slice(slice),
        }
    }

    #[inline(always)]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[inline(always)]
    pub fn push(&mut self, value: Value) {
        self.inner.push(value);
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<Value> {
        self.inner.pop()
    }

    #[inline(always)]
    pub fn truncate(&mut self, new_len: usize) {
        self.inner.truncate(new_len);
    }

    #[inline(always)]
    pub fn resize(&mut self, new_len: usize, value: Value) {
        self.inner.resize(new_len, value)
    }

    #[inline(always)]
    pub fn extend_from_slice(&mut self, slice: &[Value]) {
        self.inner.extend_from_slice(slice);
    }

    #[inline(always)]
    pub fn drain(&mut self, range: std::ops::Range<usize>) -> Vec<Value> {
        self.inner.drain(range).collect()
    }

    /// Retains only elements which f(elem) returns true.
    ///
    /// Returns true when one or some elements were removed.
    pub(crate) fn retain<F>(&mut self, mut f: F) -> Result<bool, RubyError>
    where
        F: FnMut(&Value) -> Result<bool, RubyError>,
    {
        let len = self.len();
        let mut del = 0;
        {
            let v = &mut **self;
            for i in 0..len {
                if !f(&v[i])? {
                    del += 1;
                } else if del > 0 {
                    v.swap(i - del, i);
                }
            }
        }
        if del > 0 {
            self.truncate(len - del);
        }
        Ok(del != 0)
    }
}

impl ArrayInfo {
    /// Calculate array index.
    /// if `index` is a zero or positeve integer, return `index`.
    /// Else, return `len` + `index.`
    fn get_array_index(&self, index: i64) -> Result<usize, RubyError> {
        if index < 0 {
            let i = self.len() as i64 + index;
            if i < 0 {
                return Err(RubyError::range("Index too small for array."));
            };
            Ok(i as usize)
        } else {
            Ok(index as usize)
        }
    }

    pub(crate) fn get_elem(&self, args: &[Value]) -> VMResult {
        if args.len() == 1 {
            return self.get_elem1(args[0]);
        };
        let index = args[0].coerce_to_fixnum("Index")?;
        let self_len = self.len();
        let index = self.get_array_index(index).unwrap_or(self_len);
        let len = args[1].coerce_to_fixnum("Index")?;
        let val = if len < 0 {
            Value::nil()
        } else if index >= self_len {
            Value::array_empty()
        } else {
            let len = len as usize;
            let end = std::cmp::min(self_len, index + len);
            Value::array_from_slice(&self[index..end])
        };
        Ok(val)
    }

    pub(crate) fn get_elem1(&self, idx: Value) -> VMResult {
        if let Some(index) = idx.as_fixnum() {
            let self_len = self.len();
            let index = self.get_array_index(index).unwrap_or(self_len);
            let val = self.get_elem_imm(index);
            Ok(val)
        } else if let Some(range) = idx.as_range() {
            let len = self.len() as i64;
            let i_start = match range.start.coerce_to_fixnum("Start of the range")? {
                i if i < 0 => len + i,
                i => i,
            };
            let start = if len < i_start {
                return Ok(Value::nil());
            } else if len == i_start {
                return Ok(Value::array_empty());
            } else {
                i_start as usize
            };
            let i_end = range.end.coerce_to_fixnum("End of the range")?;
            let end = if i_end >= 0 {
                let end = i_end as usize + if range.exclude { 0 } else { 1 };
                if self.len() < end {
                    self.len()
                } else {
                    end
                }
            } else {
                (len + i_end + if range.exclude { 0 } else { 1 }) as usize
            };
            if start >= end {
                return Ok(Value::array_empty());
            }
            Ok(Value::array_from(self[start..end].to_vec()))
        } else {
            Err(VMError::no_implicit_conv(idx, "Integer"))
        }
    }

    pub(crate) fn get_elem_imm(&self, index: usize) -> Value {
        if index >= self.len() {
            Value::nil()
        } else {
            self[index]
        }
    }

    pub(crate) fn set_elem(&mut self, args: &[Value]) -> VMResult {
        let val = if args.len() == 3 { args[2] } else { args[1] };
        if args.len() == 2 {
            self.set_elem1(args[0], args[1])
        } else {
            let index = args[0].coerce_to_fixnum("Index")?;
            let index = self.get_array_index(index)?;
            let length = args[1].coerce_to_fixnum("Length")?;
            if length < 0 {
                return Err(RubyError::index(format!("Negative length. {}", length)));
            };
            self.set_elem2(index, length as usize, val)
        }
    }

    pub(crate) fn set_elem1(&mut self, idx: Value, val: Value) -> VMResult {
        if let Some(index) = idx.as_fixnum() {
            if index >= 0 {
                self.set_elem_imm(index as usize, val);
            } else {
                let index = self.get_array_index(index)?;
                self[index] = val;
            }
            Ok(val)
        } else if let Some(range) = idx.as_range() {
            let first = {
                let i = range.start.coerce_to_fixnum("Start of the range")?;
                self.get_array_index(i)?
            };
            let last = {
                let i = range.end.coerce_to_fixnum("End of the range")?;
                self.get_array_index(i)? + if range.exclude { 0 } else { 1 }
            };
            if last < first {
                self.set_elem2(first, 0, val)
            } else {
                let length = last - first;
                self.set_elem2(first, length, val)
            }
        } else {
            Err(VMError::no_implicit_conv(idx, "Integer or Range"))
        }
    }

    pub(crate) fn set_elem2(&mut self, index: usize, length: usize, val: Value) -> VMResult {
        let len = self.len();
        match val.as_array() {
            Some(ary) => {
                // if self = ary, something wrong happens..
                let ary_len = ary.len();
                if index >= len || index + length > len {
                    self.resize(index + ary_len, Value::nil());
                } else if ary_len > length {
                    // possibly self == ary
                    self.resize(len + ary_len - length, Value::nil());
                    self.copy_within(index + length..len, index + ary_len);
                } else {
                    // self != ary
                    self.copy_within(index + length..len, index + ary_len);
                    self.resize(len + ary_len - length, Value::nil());
                }
                self[index..index + ary_len].copy_from_slice(&ary[0..ary_len]);
            }
            None => {
                if index >= len {
                    self.resize(index + 1, Value::nil());
                } else if length == 0 {
                    self.push(Value::nil());
                    self.copy_within(index..len, index + 1);
                } else {
                    let end = index + length;
                    if end < len {
                        self.copy_within(end..len, index + 1);
                        self.truncate(len + 1 - length);
                    } else {
                        self.truncate(index + 1);
                    }
                }
                self[index] = val;
            }
        };
        Ok(val)
    }

    pub(crate) fn set_elem_imm(&mut self, index: usize, val: Value) {
        if index >= self.len() {
            self.resize(index as usize, Value::nil());
            self.push(val);
        } else {
            self[index] = val;
        }
    }
}
