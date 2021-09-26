use crate::*;

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
    fn deref(&self) -> &Self::Target {
        match self.0.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => aref,
                _ => unreachable!(),
            },
            None => unreachable!(),
        }
    }
}

impl std::ops::DerefMut for Array {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.0.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Array(aref) => aref,
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
    pub fn new(val: Value) -> Self {
        val.as_array().unwrap();
        Array(val)
    }

    pub fn new_unchecked(val: Value) -> Self {
        Array(val)
    }

    /*pub fn default() -> Self {
        Array(Value::nil())
    }*/

    fn get(self) -> Value {
        self.0
    }

    pub fn id(self) -> u64 {
        self.0.id()
    }

    pub fn shallow_dup(&self) -> Self {
        Array(self.get().shallow_dup())
    }
}

#[derive(Debug, Clone)]
pub struct ArrayInfo {
    pub elements: Vec<Value>,
}

impl GC for ArrayInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.elements.iter().for_each(|v| v.mark(alloc));
    }
}

impl std::ops::Deref for ArrayInfo {
    type Target = Vec<Value>;
    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}

impl std::ops::DerefMut for ArrayInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elements
    }
}

impl ArrayInfo {
    pub fn new(elements: Vec<Value>) -> Self {
        ArrayInfo { elements }
    }

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

    pub fn get_elem(&self, args: &Args) -> VMResult {
        args.check_args_range(1, 2)?;
        if args.len() == 1 {
            return self.get_elem1(args[0]);
        };
        let index = args[0].coerce_to_fixnum("Index")?;
        let self_len = self.elements.len();
        let index = self.get_array_index(index).unwrap_or(self_len);
        let len = args[1].coerce_to_fixnum("Index")?;
        let val = if len < 0 {
            Value::nil()
        } else if index >= self_len {
            Value::array_empty()
        } else {
            let len = len as usize;
            let end = std::cmp::min(self_len, index + len);
            let ary = self.elements[index..end].to_vec();
            Value::array_from(ary)
        };
        Ok(val)
    }

    pub fn get_elem1(&self, idx: Value) -> VMResult {
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
            Ok(Value::array_from(self.elements[start..end].to_vec()))
        } else {
            Err(RubyError::no_implicit_conv(idx, "Integer"))
        }
    }

    pub fn get_elem_imm(&self, index: usize) -> Value {
        if index >= self.elements.len() {
            Value::nil()
        } else {
            self.elements[index]
        }
    }

    pub fn set_elem(&mut self, args: &Args) -> VMResult {
        args.check_args_range(2, 3)?;
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

    pub fn set_elem1(&mut self, idx: Value, val: Value) -> VMResult {
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
            Err(RubyError::no_implicit_conv(idx, "Integer or Range"))
        }
    }

    pub fn set_elem2(&mut self, index: usize, length: usize, val: Value) -> VMResult {
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

    pub fn set_elem_imm(&mut self, index: usize, val: Value) {
        if index >= self.len() {
            self.resize(index as usize, Value::nil());
            self.push(val);
        } else {
            self[index] = val;
        }
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn retain<F>(&mut self, mut f: F) -> Result<(), RubyError>
    where
        F: FnMut(&Value) -> Result<bool, RubyError>,
    {
        let len = self.len();
        let mut del = 0;
        {
            let v = &mut *self.elements;

            for i in 0..len {
                if !f(&v[i])? {
                    del += 1;
                } else if del > 0 {
                    v.swap(i - del, i);
                }
            }
        }
        if del > 0 {
            self.elements.truncate(len - del);
        }
        Ok(())
    }
}
