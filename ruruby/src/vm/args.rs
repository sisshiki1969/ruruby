use crate::*;
use std::ops::{Deref, DerefMut};
use std::ops::{Index, IndexMut, Range};

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Block(FnId, Frame),
    Proc(Value),
    Sym(IdentId),
}

impl From<Value> for Block {
    fn from(proc_obj: Value) -> Self {
        Self::Proc(proc_obj)
    }
}

impl From<IdentId> for Block {
    fn from(sym: IdentId) -> Self {
        Self::Sym(sym)
    }
}

impl Block {
    pub(crate) fn decode(val: Value) -> Option<Self> {
        if let Some(i) = val.as_fixnum() {
            if i == 0 {
                None
            } else {
                let u = i as u64;
                let method = FnId::from((u >> 32) as u32);
                let frame = Frame(u as u32);
                Some(Block::Block(method, frame))
            }
        } else if let Some(id) = val.as_symbol() {
            Some(id.into())
        } else {
            Some(val.into())
        }
    }

    pub(crate) fn encode(&self) -> Value {
        match self {
            Block::Proc(p) => *p,
            Block::Block(m, f) => {
                let m: u32 = (*m).into();
                let f = f.0 as u64;
                Value::fixnum((((m as u64) << 32) + f) as i64)
            }
            Block::Sym(id) => Value::symbol(*id),
        }
    }
}

impl GC<RValue> for Block {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        match self {
            Block::Proc(v) => v.mark(alloc),
            _ => {}
        }
    }
}

impl Block {
    pub(crate) fn to_iseq(&self, globals: &Globals) -> ISeqRef {
        let id = match self {
            Block::Proc(val) => {
                val.as_proc()
                    .unwrap_or_else(|| {
                        unimplemented!("Block argument must be Proc. given:{:?}", val)
                    })
                    .method
            }
            Block::Block(method, _) => *method,
            _ => unreachable!(),
        };
        globals.methods[id].as_iseq()
    }
}

#[derive(Debug, Clone)]
pub struct Args2 {
    pub block: Option<Block>,
    pub kw_arg: Value,
    args_len: usize,
}

impl Args2 {
    #[inline(always)]
    pub(crate) fn new(args_len: usize) -> Self {
        Self {
            block: None,
            kw_arg: Value::nil(),
            args_len,
        }
    }

    #[inline(always)]
    pub(crate) fn new_with_block(args_len: usize, block: impl Into<Option<Block>>) -> Self {
        Self {
            block: block.into(),
            kw_arg: Value::nil(),
            args_len,
        }
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.args_len
    }

    #[inline(always)]
    pub(crate) fn set_len(&mut self, new_len: usize) {
        self.args_len = new_len;
    }

    pub(crate) fn from(args: &Args) -> Self {
        Self {
            block: args.block.clone(),
            kw_arg: args.kw_arg,
            args_len: args.len(),
        }
    }

    pub(crate) fn append(&mut self, slice: &[Value]) {
        self.args_len += slice.len();
    }

    pub(crate) fn into(&self, vm: &VM) -> Args {
        let stack = vm.args();
        let mut arg = Args::from_slice(stack);
        arg.block = self.block.clone();
        arg.kw_arg = self.kw_arg;
        arg
    }

    pub(crate) fn check_args_num(&self, num: usize) -> Result<(), RubyError> {
        let len = self.len();
        if len == num {
            Ok(())
        } else {
            Err(RubyError::argument_wrong(len, num))
        }
    }

    pub(crate) fn check_args_range(&self, min: usize, max: usize) -> Result<(), RubyError> {
        let len = self.len();
        if min <= len && len <= max {
            Ok(())
        } else {
            Err(RubyError::argument_wrong_range(len, min, max))
        }
    }

    pub(crate) fn check_args_min(&self, min: usize) -> Result<(), RubyError> {
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

    pub(crate) fn expect_block(&self) -> Result<&Block, RubyError> {
        match &self.block {
            None => Err(RubyError::argument("Currently, needs block.")),
            Some(block) => Ok(block),
        }
    }

    pub(crate) fn expect_no_block(&self) -> Result<(), RubyError> {
        match &self.block {
            None => Ok(()),
            _ => Err(RubyError::argument("Currently, block is not supported.")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Args {
    pub block: Option<Block>,
    kw_arg: Value,
    elems: Vec<Value>,
}

impl GC<RValue> for Args {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
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
    pub(crate) fn new(len: usize) -> Self {
        Args {
            block: None,
            kw_arg: Value::nil(),
            elems: vec![Value::nil(); len],
        }
    }

    pub(crate) fn from_slice(data: &[Value]) -> Self {
        Args {
            block: None,
            kw_arg: Value::nil(),
            elems: data.to_vec(),
        }
    }

    pub(crate) fn new0() -> Self {
        Args {
            block: None,
            kw_arg: Value::nil(),
            elems: vec![],
        }
    }

    pub(crate) fn new1(arg: Value) -> Self {
        Args {
            block: None,
            kw_arg: Value::nil(),
            elems: vec![arg],
        }
    }

    pub(crate) fn new2(arg0: Value, arg1: Value) -> Self {
        Args {
            block: None,
            kw_arg: Value::nil(),
            elems: vec![arg0, arg1],
        }
    }
}

impl Args {
    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.elems.len()
    }
}

impl Index<usize> for Args {
    type Output = Value;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        &self.elems[index]
    }
}

impl Index<Range<usize>> for Args {
    type Output = [Value];
    #[inline(always)]
    fn index(&self, range: Range<usize>) -> &Self::Output {
        &self.elems[range]
    }
}

impl IndexMut<Range<usize>> for Args {
    #[inline(always)]
    fn index_mut(&mut self, range: Range<usize>) -> &mut Self::Output {
        &mut self.elems[range]
    }
}

impl IndexMut<usize> for Args {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.elems[index]
    }
}

impl Deref for Args {
    type Target = [Value];
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.elems.deref()
    }
}

impl DerefMut for Args {
    #[inline(always)]
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
}
