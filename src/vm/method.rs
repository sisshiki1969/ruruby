use crate::*;
use std::cell::RefCell;

thread_local!(
    pub static METHODS: RefCell<MethodRepo> = RefCell::new(MethodRepo::new());
);

pub struct MethodRepo {
    table: Vec<MethodInfo>,
}

impl std::ops::Index<MethodId> for MethodRepo {
    type Output = MethodInfo;
    fn index(&self, id: MethodId) -> &MethodInfo {
        &self.table[id.0.get() as usize]
    }
}

impl std::ops::IndexMut<MethodId> for MethodRepo {
    fn index_mut(&mut self, id: MethodId) -> &mut MethodInfo {
        &mut self.table[id.0.get() as usize]
    }
}

impl MethodRepo {
    pub fn new() -> Self {
        Self {
            table: vec![
                MethodInfo::Void, // dummy
                MethodInfo::Void, // default
                MethodInfo::BuiltinFunc {
                    func: enumerator_iterate,
                    name: IdentId::_ENUM_FUNC,
                }, // METHOD_ENUM
            ],
        }
    }

    pub fn add(info: MethodInfo) -> MethodId {
        METHODS.with(|m| {
            let table = &mut m.borrow_mut().table;
            table.push(info);
            MethodId::new((table.len() - 1) as u32)
        })
    }

    pub fn update(id: MethodId, info: MethodInfo) {
        METHODS.with(|m| {
            m.borrow_mut()[id] = info;
        })
    }

    pub fn get(id: MethodId) -> MethodInfo {
        METHODS.with(|m| m.borrow()[id].clone())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MethodId(std::num::NonZeroU32);

impl std::default::Default for MethodId {
    fn default() -> Self {
        Self::new(1)
    }
}

impl MethodId {
    fn new(id: u32) -> Self {
        Self(std::num::NonZeroU32::new(id).unwrap())
    }

    pub fn as_iseq(&self) -> ISeqRef {
        METHODS.with(|m| m.borrow()[*self].as_iseq())
    }
}

impl From<u64> for MethodId {
    fn from(id: u64) -> Self {
        Self::new(id as u32)
    }
}

impl Into<u64> for MethodId {
    fn into(self) -> u64 {
        self.0.get() as u64
    }
}

impl From<u32> for MethodId {
    fn from(id: u32) -> Self {
        Self::new(id)
    }
}

impl Into<usize> for MethodId {
    fn into(self) -> usize {
        self.0.get() as usize
    }
}

pub type BuiltinFunc = fn(vm: &mut VM, self_val: Value, args: &Args) -> VMResult;

pub type MethodTable = FxHashMap<IdentId, MethodId>;

pub static METHOD_ENUM: MethodId = MethodId(unsafe { std::num::NonZeroU32::new_unchecked(2) });

#[derive(Clone)]
pub enum MethodInfo {
    RubyFunc { iseq: ISeqRef },
    AttrReader { id: IdentId },
    AttrWriter { id: IdentId },
    BuiltinFunc { name: IdentId, func: BuiltinFunc },
    Void,
}

impl GC for MethodInfo {
    fn mark(&self, alloc: &mut Allocator) {
        match self {
            MethodInfo::RubyFunc { iseq } => iseq.class_defined.iter().for_each(|c| c.mark(alloc)),
            _ => return,
        };
    }
}

impl std::fmt::Debug for MethodInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodInfo::RubyFunc { iseq } => write!(f, "RubyFunc {:?}", *iseq),
            MethodInfo::AttrReader { id } => write!(f, "AttrReader {:?}", id),
            MethodInfo::AttrWriter { id } => write!(f, "AttrWriter {:?}", id),
            MethodInfo::BuiltinFunc { name, .. } => write!(f, "BuiltinFunc {:?}", name),
            MethodInfo::Void => write!(f, "Void"),
        }
    }
}

impl Default for MethodInfo {
    fn default() -> Self {
        MethodInfo::Void
    }
}

impl MethodInfo {
    pub fn as_iseq(&self) -> ISeqRef {
        if let MethodInfo::RubyFunc { iseq } = self {
            *iseq
        } else {
            unimplemented!("Methodref is illegal.")
        }
    }
}

//----------------------------------------------------------------------------------

#[derive(Default, Debug, Clone)]
pub struct ISeqParams {
    pub param_ident: Vec<IdentId>,
    pub req: usize,
    pub opt: usize,
    pub rest: Option<bool>, // Some(true): exists and bind to param, Some(false): exists but to be discarded, None: not exists.
    pub post: usize,
    pub block: bool,
    pub keyword: FxHashMap<IdentId, LvarId>,
    pub kwrest: bool,
}

impl ISeqParams {
    pub fn is_opt(&self) -> bool {
        self.opt == 0
            && self.rest.is_none()
            && self.post == 0
            && !self.block
            && self.keyword.is_empty()
            && !self.kwrest
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ISeqKind {
    Other,           // eval or unnamed method
    Method(IdentId), // method or lambda
    Block,           // block or proc
}

impl Default for ISeqKind {
    fn default() -> Self {
        ISeqKind::Other
    }
}

pub type ISeqRef = Ref<ISeqInfo>;

#[derive(Debug, Clone, Default)]
pub struct ISeqInfo {
    pub method: MethodId,
    pub name: Option<IdentId>,
    pub params: ISeqParams,
    pub iseq: ISeq,
    pub lvar: LvarCollector,
    pub lvars: usize,
    /// This flag is set when the following conditions are met.
    /// - Has no optional/post/rest/block/keyword parameters.
    pub opt_flag: bool,
    /// The Class where this method was described.
    /// This field is set to None when IseqInfo was created by Codegen.
    /// Later, when the VM execute Inst::DEF_METHOD or DEF_SMETHOD,
    /// Set to Some() in class definition context, or None in the top level.
    pub exception_table: Vec<ExceptionEntry>,
    pub class_defined: Vec<Module>,
    pub iseq_sourcemap: Vec<(ISeqPos, Loc)>,
    pub source_info: SourceInfoRef,
    pub kind: ISeqKind,
    pub forvars: Vec<(u32, u32)>,
}

impl ISeqInfo {
    pub fn new(
        method: MethodId,
        name: Option<IdentId>,
        params: ISeqParams,
        iseq: ISeq,
        lvar: LvarCollector,
        exception_table: Vec<ExceptionEntry>,
        iseq_sourcemap: Vec<(ISeqPos, Loc)>,
        source_info: SourceInfoRef,
        kind: ISeqKind,
        forvars: Vec<(u32, u32)>,
    ) -> Self {
        let lvars = lvar.len();
        let opt_flag = params.is_opt();
        ISeqInfo {
            method,
            name,
            params,
            iseq,
            lvar,
            lvars,
            exception_table,
            opt_flag,
            class_defined: vec![],
            iseq_sourcemap,
            source_info,
            kind,
            forvars,
        }
    }

    pub fn is_block(&self) -> bool {
        match self.kind {
            ISeqKind::Block => true,
            _ => false,
        }
    }

    pub fn is_method(&self) -> bool {
        match self.kind {
            ISeqKind::Method(_) => true,
            _ => false,
        }
    }
}

//----------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodObjInfo {
    pub name: IdentId,
    pub receiver: Value,
    pub method: MethodId,
}

impl MethodObjInfo {
    pub fn new(name: IdentId, receiver: Value, method: MethodId) -> Self {
        MethodObjInfo {
            name,
            receiver,
            method,
        }
    }
}

impl GC for MethodObjInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.receiver.mark(alloc);
    }
}
