use crate::*;
use std::ops::{Deref, DerefMut};
use std::ops::{Index, IndexMut, Range};

const ARG_ARRAY_SIZE: usize = 8;

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Block(MethodId, Context),
    Proc(Value),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Context {
    Frame(Frame),
    Heap(HeapCtxRef),
}

impl From<Frame> for Context {
    fn from(frame: Frame) -> Self {
        Self::Frame(frame)
    }
}

impl From<HeapCtxRef> for Context {
    fn from(ctx: HeapCtxRef) -> Self {
        Self::Heap(ctx)
    }
}

impl Context {
    pub fn get_current(&self) -> Self {
        match self {
            Self::Frame(f) => (*f).into(),
            Self::Heap(c) => (*c).into(),
        }
    }
}

impl GC for Block {
    fn mark(&self, alloc: &mut Allocator) {
        match self {
            Block::Block(_, Context::Heap(ctx)) => {
                ctx.mark(alloc);
            }
            Block::Proc(v) => v.mark(alloc),
            _ => {}
        }
    }
}

impl Block {
    pub fn to_iseq(&self) -> ISeqRef {
        match self {
            Block::Proc(val) => {
                val.as_proc()
                    .unwrap_or_else(|| {
                        unimplemented!("Block argument must be Proc. given:{:?}", val)
                    })
                    .iseq
            }
            Block::Block(method, _) => method.as_iseq(),
        }
    }

    pub fn create_context(&self, vm: &mut VM) -> HeapCtxRef {
        match self {
            Block::Block(method, outer) => vm.create_block_context(*method, outer.clone()),
            Block::Proc(proc) => {
                let pinfo = proc.as_proc().unwrap();
                HeapCtxRef::new_heap(pinfo.self_val, None, pinfo.iseq, pinfo.outer)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Args2 {
    pub block: Option<Block>,
    pub kw_arg: Value,
    args_len: usize,
}

impl Args2 {
    pub fn new(args_len: usize) -> Self {
        Self {
            block: None,
            kw_arg: Value::nil(),
            args_len,
        }
    }

    pub fn new_with_block(args_len: usize, block: impl Into<Option<Block>>) -> Self {
        Self {
            block: block.into(),
            kw_arg: Value::nil(),
            args_len,
        }
    }

    pub fn len(&self) -> usize {
        self.args_len
    }

    pub fn from(args: &Args) -> Self {
        Self {
            block: args.block.clone(),
            kw_arg: args.kw_arg,
            args_len: args.len(),
        }
    }

    pub fn append(&mut self, slice: &[Value]) {
        self.args_len += slice.len();
    }

    pub fn into(&self, vm: &VM) -> Args {
        let stack = vm.args();
        let mut arg = Args::from_slice(stack);
        arg.block = self.block.clone();
        arg.kw_arg = self.kw_arg;
        arg
    }

    pub fn check_args_num(&self, num: usize) -> Result<(), RubyError> {
        let len = self.len();
        if len == num {
            Ok(())
        } else {
            Err(RubyError::argument_wrong(len, num))
        }
    }

    pub fn check_args_range(&self, min: usize, max: usize) -> Result<(), RubyError> {
        let len = self.len();
        if min <= len && len <= max {
            Ok(())
        } else {
            Err(RubyError::argument_wrong_range(len, min, max))
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
            None => Err(RubyError::argument("Currently, needs block.")),
            Some(block) => Ok(block),
        }
    }

    pub fn expect_no_block(&self) -> Result<(), RubyError> {
        match &self.block {
            None => Ok(()),
            _ => Err(RubyError::argument("Currently, block is not supported.")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Args {
    pub block: Option<Block>,
    pub kw_arg: Value,
    elems: SmallVec<[Value; ARG_ARRAY_SIZE]>,
}

impl GC for Args {
    fn mark(&self, alloc: &mut Allocator) {
        for arg in self.iter() {
            arg.mark(alloc);
        }
        self.kw_arg.mark(alloc);
        if let Some(b) = &self.block {
            b.mark(alloc)
        };
    }
}

// Constructors for Args
impl Args {
    pub fn new(len: usize) -> Self {
        Args {
            block: None,
            kw_arg: Value::nil(),
            elems: smallvec![Value::nil(); len],
        }
    }

    pub fn from_slice(data: &[Value]) -> Self {
        Args {
            block: None,
            kw_arg: Value::nil(),
            elems: SmallVec::from_slice(data),
        }
    }

    pub fn new0() -> Self {
        Args {
            block: None,
            kw_arg: Value::nil(),
            elems: smallvec![],
        }
    }

    pub fn new0_block(block: Block) -> Self {
        Args {
            block: Some(block),
            kw_arg: Value::nil(),
            elems: smallvec![],
        }
    }

    pub fn new1(arg: Value) -> Self {
        Args {
            block: None,
            kw_arg: Value::nil(),
            elems: smallvec![arg],
        }
    }

    pub fn new2(arg0: Value, arg1: Value) -> Self {
        Args {
            block: None,
            kw_arg: Value::nil(),
            elems: smallvec![arg0, arg1],
        }
    }

    pub fn new3(block: impl Into<Option<Block>>, arg0: Value, arg1: Value, arg2: Value) -> Self {
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
    fn args1() {
        let args = Args::new1(Value::integer(0));
        assert_eq!(0, args[0].as_fixnum().unwrap());
    }

    #[test]
    fn args2() {
        let args = Args::new2(Value::integer(0), Value::integer(1));
        assert_eq!(0, args[0].as_fixnum().unwrap());
        assert_eq!(1, args[1].as_fixnum().unwrap());
    }

    #[test]
    fn args3() {
        let args = Args::new3(
            None,
            Value::integer(0),
            Value::integer(1),
            Value::integer(2),
        );
        assert_eq!(0, args[0].as_fixnum().unwrap());
        assert_eq!(1, args[1].as_fixnum().unwrap());
        assert_eq!(2, args[2].as_fixnum().unwrap());
    }
}
