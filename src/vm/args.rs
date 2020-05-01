use crate::*;
use std::ops::Deref;
use std::ops::{Index, IndexMut, Range};

const VEC_ARRAY_SIZE: usize = 8;

#[derive(Debug, Clone)]
pub struct Args {
    pub block: Option<MethodRef>,
    pub kw_arg: Option<Value>,
    elems: ArgsArray,
}

impl Args {
    pub fn new(len: usize) -> Self {
        Args {
            block: None,
            kw_arg: None,
            elems: ArgsArray::new(len),
        }
    }

    pub fn push(&mut self, val: Value) {
        self.elems.push(val);
    }

    pub fn new0() -> Self {
        Args {
            block: None,
            kw_arg: None,
            elems: ArgsArray::new0(),
        }
    }

    pub fn new1(arg: Value) -> Self {
        Args {
            block: None,
            kw_arg: None,
            elems: ArgsArray::new1(arg),
        }
    }

    pub fn new2(arg0: Value, arg1: Value) -> Self {
        Args {
            block: None,
            kw_arg: None,
            elems: ArgsArray::new2(arg0, arg1),
        }
    }

    pub fn new3(
        block: impl Into<Option<MethodRef>>,
        arg0: Value,
        arg1: Value,
        arg2: Value,
    ) -> Self {
        Args {
            block: block.into(),
            kw_arg: None,
            elems: ArgsArray::new3(arg0, arg1, arg2),
        }
    }

    pub fn len(&self) -> usize {
        self.elems.len()
    }
}

impl Index<usize> for Args {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        &self.elems[index]
    }
}

impl Index<Range<usize>> for Args {
    type Output = [Value];

    fn index(&self, range: Range<usize>) -> &Self::Output {
        &self.elems[range]
    }
}

impl IndexMut<usize> for Args {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.elems[index]
    }
}

impl Deref for Args {
    type Target = [Value];
    fn deref(&self) -> &Self::Target {
        self.elems.deref()
    }
}

#[derive(Debug, Clone)]
enum ArgsArray {
    Array {
        len: usize,
        ary: [Value; VEC_ARRAY_SIZE],
    },
    Vec(Vec<Value>),
}

impl ArgsArray {
    fn new(len: usize) -> Self {
        if len <= VEC_ARRAY_SIZE {
            ArgsArray::Array {
                len,
                ary: [Value::uninitialized(); VEC_ARRAY_SIZE],
            }
        } else {
            ArgsArray::Vec(vec![Value::uninitialized(); len])
        }
    }

    fn push(&mut self, val: Value) {
        if self.len() == VEC_ARRAY_SIZE {
            let mut ary = self[0..VEC_ARRAY_SIZE].to_vec();
            ary.push(val);
            std::mem::replace(self, ArgsArray::Vec(ary));
        } else {
            match self {
                ArgsArray::Vec(ref mut v) => v.push(val),
                ArgsArray::Array {
                    ref mut len,
                    ref mut ary,
                } => {
                    ary[*len] = val;
                    *len += 1;
                }
            }
        }
    }

    fn new0() -> Self {
        ArgsArray::Array {
            len: 0,
            ary: [Value::uninitialized(); VEC_ARRAY_SIZE],
        }
    }

    fn new1(arg: Value) -> Self {
        let mut ary = [Value::uninitialized(); VEC_ARRAY_SIZE];
        ary[0] = arg;
        ArgsArray::Array { len: 1, ary }
    }

    fn new2(arg0: Value, arg1: Value) -> Self {
        let mut ary = [Value::uninitialized(); VEC_ARRAY_SIZE];
        ary[0] = arg0;
        ary[1] = arg1;
        ArgsArray::Array { len: 2, ary }
    }

    fn new3(arg0: Value, arg1: Value, arg2: Value) -> Self {
        let mut ary = [Value::uninitialized(); VEC_ARRAY_SIZE];
        ary[0] = arg0;
        ary[1] = arg1;
        ary[2] = arg2;
        ArgsArray::Array { len: 3, ary }
    }

    fn len(&self) -> usize {
        match self {
            ArgsArray::Array { len, .. } => *len,
            ArgsArray::Vec(v) => v.len(),
        }
    }
}

impl Index<usize> for ArgsArray {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            ArgsArray::Array { ary, .. } => &ary[index],
            ArgsArray::Vec(v) => &v[index],
        }
    }
}

impl Index<Range<usize>> for ArgsArray {
    type Output = [Value];

    fn index(&self, range: Range<usize>) -> &Self::Output {
        match self {
            ArgsArray::Array { ary, .. } => &ary[range],
            ArgsArray::Vec(v) => &v[range],
        }
    }
}

impl IndexMut<usize> for ArgsArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self {
            ArgsArray::Array { ary, .. } => &mut ary[index],
            ArgsArray::Vec(v) => &mut v[index],
        }
    }
}

impl Deref for ArgsArray {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        match self {
            ArgsArray::Array { len, ary } => &ary[0..*len],
            ArgsArray::Vec(v) => &v,
        }
    }
}
