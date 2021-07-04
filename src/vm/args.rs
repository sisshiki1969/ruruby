use crate::*;
use std::ops::{Deref, DerefMut};
use std::ops::{Index, IndexMut, Range};

const ARG_ARRAY_SIZE: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Block(MethodId, ContextRef),
    Proc(Value),
    None,
}

impl GC for Block {
    fn mark(&self, alloc: &mut Allocator) {
        match self {
            Block::Block(_, ctx) => ctx.mark(alloc),
            Block::Proc(v) => v.mark(alloc),
            Block::None => {}
        }
    }
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

    pub fn from_u32(id: u32, vm: &mut VM) -> Self {
        match id {
            0 => Block::None,
            i => vm.new_block(i),
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

impl GC for Args {
    fn mark(&self, alloc: &mut Allocator) {
        for arg in self.iter() {
            arg.mark(alloc);
        }
        self.kw_arg.mark(alloc);
        self.block.mark(alloc);
    }
}

// Constructors for Args
impl Args {
    pub fn new(len: usize) -> Self {
        Args {
            block: Block::None,
            kw_arg: Value::nil(),
            elems: smallvec![Value::nil(); len],
        }
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

    pub fn new0_block(block: Block) -> Self {
        Args {
            block: block,
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
}

impl Args {
    pub fn len(&self) -> usize {
        self.elems.len()
    }

    pub fn push(&mut self, val: Value) {
        self.elems.push(val);
    }

    pub fn append(&mut self, slice: &[Value]) {
        self.elems.extend_from_slice(slice);
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
            Block::None => Err(RubyError::argument("Currently, needs block.")),
            block => Ok(block),
        }
    }

    pub fn expect_no_block(&self) -> Result<(), RubyError> {
        match &self.block {
            Block::None => Ok(()),
            _ => Err(RubyError::argument("Currently, block is not supported.")),
        }
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

impl IndexMut<Range<usize>> for Args {
    fn index_mut(&mut self, range: Range<usize>) -> &mut Self::Output {
        &mut self.elems[range]
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
        assert!(Value::false_val().eq(&args[3]));
        assert!(Value::true_val().eq(&args[17]));
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
