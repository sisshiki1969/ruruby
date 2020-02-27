use super::method::MethodRef;
use super::value::Value;
use std::ops::Deref;
use std::ops::{Index, IndexMut};

const VEC_ARRAY_SIZE: usize = 8;

#[derive(Debug, Clone)]
pub struct Args {
    pub self_value: Value,
    pub block: Option<MethodRef>,
    args: ArgsArray,
}

impl Args {
    pub fn new(len: usize) -> Self {
        Args {
            self_value: Value::nil(),
            block: None,
            args: ArgsArray::new(len),
        }
    }

    pub fn push(&mut self, val: Value) {
        self.args.push(val);
    }

    pub fn new0(self_value: Value, block: impl Into<Option<MethodRef>>) -> Self {
        Args {
            self_value,
            block: block.into(),
            args: ArgsArray::new0(),
        }
    }

    pub fn new1(self_value: Value, block: impl Into<Option<MethodRef>>, arg: Value) -> Self {
        Args {
            self_value,
            block: block.into(),
            args: ArgsArray::new1(arg),
        }
    }

    pub fn new2(
        self_value: Value,
        block: impl Into<Option<MethodRef>>,
        arg0: Value,
        arg1: Value,
    ) -> Self {
        Args {
            self_value,
            block: block.into(),
            args: ArgsArray::new2(arg0, arg1),
        }
    }

    pub fn new3(
        self_value: Value,
        block: impl Into<Option<MethodRef>>,
        arg0: Value,
        arg1: Value,
        arg2: Value,
    ) -> Self {
        Args {
            self_value,
            block: block.into(),
            args: ArgsArray::new3(arg0, arg1, arg2),
        }
    }

    pub fn new4(
        self_value: Value,
        block: impl Into<Option<MethodRef>>,
        arg0: Value,
        arg1: Value,
        arg2: Value,
        arg3: Value,
    ) -> Self {
        Args {
            self_value,
            block: block.into(),
            args: ArgsArray::new4(arg0, arg1, arg2, arg3),
        }
    }

    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn get_slice(&self, start: usize, end: usize) -> &[Value] {
        self.args.get_slice(start, end)
    }
}

impl Index<usize> for Args {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        &self.args[index]
    }
}

impl IndexMut<usize> for Args {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.args.index_mut(index)
    }
}

impl Deref for Args {
    type Target = [Value];

    fn deref(&self) -> &Self::Target {
        self.args.deref()
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
            let mut ary = self.get_slice(0, VEC_ARRAY_SIZE).to_vec();
            ary.push(val);
            unsafe { std::ptr::write(self, ArgsArray::Vec(ary)) };
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

    fn new4(arg0: Value, arg1: Value, arg2: Value, arg3: Value) -> Self {
        let mut ary = [Value::uninitialized(); VEC_ARRAY_SIZE];
        ary[0] = arg0;
        ary[1] = arg1;
        ary[2] = arg2;
        ary[3] = arg3;
        ArgsArray::Array { len: 4, ary }
    }

    fn len(&self) -> usize {
        match self {
            ArgsArray::Array { len, .. } => *len,
            ArgsArray::Vec(v) => v.len(),
        }
    }

    fn get_slice(&self, start: usize, end: usize) -> &[Value] {
        match self {
            ArgsArray::Array { ary, .. } => &ary[start..end],
            ArgsArray::Vec(v) => &v[start..end],
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
