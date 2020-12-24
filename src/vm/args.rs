use crate::*;
use std::ops::{Deref, DerefMut};
use std::ops::{Index, IndexMut, Range};

const ARG_ARRAY_SIZE: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Block(MethodRef, ContextRef),
    Proc(Value),
    None,
}

impl Block {
    pub fn to_iseq(&self) -> ISeqRef {
        match self {
            Block::Proc(val) => {
                //let val = *val;
                val.as_proc()
                    .unwrap_or_else(|| {
                        unimplemented!("Block argument must be Proc. given:{:?}", val)
                    })
                    .context
                    .iseq_ref
                    .unwrap()
            }
            Block::Block(methodref, _) => methodref.as_iseq(),
            Block::None => unreachable!(),
        }
    }

    pub fn is_none(&self) -> bool {
        *self == Block::None
    }

    pub fn is_some(&self) -> bool {
        !self.is_none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub block: Block,
    pub kw_arg: Value,
    elems: SmallVec<[Value; ARG_ARRAY_SIZE]>,
}

impl Args {
    pub fn new(len: usize) -> Self {
        Args {
            block: Block::None,
            kw_arg: Value::nil(),
            elems: smallvec![Value::nil(); len],
        }
    }

    pub fn push(&mut self, val: Value) {
        self.elems.push(val);
    }

    pub fn from_slice(data: &[Value]) -> Self {
        Args {
            block: Block::None,
            kw_arg: Value::nil(),
            elems: SmallVec::from_slice(data),
        }
    }

    pub fn new0() -> Self {
        Args {
            block: Block::None,
            kw_arg: Value::nil(),
            elems: smallvec![],
        }
    }

    pub fn new1(arg: Value) -> Self {
        Args {
            block: Block::None,
            kw_arg: Value::nil(),
            elems: smallvec![arg],
        }
    }

    pub fn new2(arg0: Value, arg1: Value) -> Self {
        Args {
            block: Block::None,
            kw_arg: Value::nil(),
            elems: smallvec![arg0, arg1],
        }
    }

    pub fn new3(block: impl Into<Block>, arg0: Value, arg1: Value, arg2: Value) -> Self {
        Args {
            block: block.into(),
            kw_arg: Value::nil(),
            elems: smallvec![arg0, arg1, arg2],
        }
    }

    pub fn len(&self) -> usize {
        self.elems.len()
    }

    pub fn into_vec(self) -> Vec<Value> {
        self.elems.into_vec()
    }

    pub fn check_args_num(&self, num: usize) -> Result<(), RubyError> {
        let len = self.len();
        if len == num {
            Ok(())
        } else {
            Err(RubyError::argument(format!(
                "Wrong number of arguments. (given {}, expected {})",
                len, num
            )))
        }
    }

    pub fn check_args_range(&self, min: usize, max: usize) -> Result<(), RubyError> {
        let len = self.len();
        if min <= len && len <= max {
            Ok(())
        } else {
            Err(RubyError::argument(format!(
                "Wrong number of arguments. (given {}, expected {}..{})",
                len, min, max
            )))
        }
    }

    pub fn check_args_range_ofs(
        &self,
        offset: usize,
        min: usize,
        max: usize,
    ) -> Result<(), RubyError> {
        let len = self.len() + offset;
        if min <= len && len <= max {
            Ok(())
        } else {
            Err(RubyError::argument(format!(
                "Wrong number of arguments. (given {}, expected {}..{})",
                len, min, max
            )))
        }
    }

    pub fn check_args_min(&self, min: usize) -> Result<(), RubyError> {
        let len = self.len();
        if min <= len {
            Ok(())
        } else {
            Err(RubyError::argument(format!(
                "Wrong number of arguments. (given {}, expected {}+)",
                len, min
            )))
        }
    }

    pub fn expect_block(&self) -> Result<&Block, RubyError> {
        match &self.block {
            Block::None => return Err(RubyError::argument("Currently, needs block.")),
            block => Ok(block),
        }
    }
}

impl Index<usize> for Args {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { self.elems.get_unchecked(index) }
    }
}

impl Index<Range<usize>> for Args {
    type Output = [Value];

    fn index(&self, range: Range<usize>) -> &Self::Output {
        unsafe { self.elems.get_unchecked(range) }
    }
}

impl IndexMut<Range<usize>> for Args {
    fn index_mut(&mut self, range: Range<usize>) -> &mut Self::Output {
        unsafe { self.elems.get_unchecked_mut(range) }
    }
}

impl IndexMut<usize> for Args {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { self.elems.get_unchecked_mut(index) }
    }
}

impl Deref for Args {
    type Target = [Value];
    fn deref(&self) -> &Self::Target {
        self.elems.deref()
    }
}

impl DerefMut for Args {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.elems.deref_mut()
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn args() {
        let mut args = Args::new(0);
        for i in 0..20 {
            args.push(Value::integer(i as i64));
        }
        for i in 0..20 {
            assert_eq!(i as i64, args[i].as_integer().unwrap());
        }
        args[3] = Value::false_val();
        args[17] = Value::true_val();
        assert_eq!(Value::false_val(), args[3]);
        assert_eq!(Value::true_val(), args[17]);
    }

    #[test]
    fn args1() {
        let args = Args::new1(Value::integer(0));
        assert_eq!(0, args[0].as_integer().unwrap());
    }

    #[test]
    fn args2() {
        let args = Args::new2(Value::integer(0), Value::integer(1));
        assert_eq!(0, args[0].as_integer().unwrap());
        assert_eq!(1, args[1].as_integer().unwrap());
    }

    #[test]
    fn args3() {
        let args = Args::new3(
            Block::None,
            Value::integer(0),
            Value::integer(1),
            Value::integer(2),
        );
        assert_eq!(0, args[0].as_integer().unwrap());
        assert_eq!(1, args[1].as_integer().unwrap());
        assert_eq!(2, args[2].as_integer().unwrap());
    }
}
