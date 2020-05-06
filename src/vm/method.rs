use crate::*;
use std::collections::HashMap;

pub type BuiltinFunc = fn(vm: &mut VM, self_val: Value, args: &Args) -> VMResult;

pub type MethodTable = HashMap<IdentId, MethodRef>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodRef(u32);

impl std::hash::Hash for MethodRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Into<u32> for MethodRef {
    fn into(self) -> u32 {
        self.0
    }
}

impl From<u32> for MethodRef {
    fn from(id: u32) -> Self {
        MethodRef(id)
    }
}

impl MethodRef {
    pub fn is_none(&self) -> bool {
        self.0 == 0
    }
}

#[derive(Clone)]
pub enum MethodInfo {
    RubyFunc { iseq: ISeqRef },
    AttrReader { id: IdentId },
    AttrWriter { id: IdentId },
    BuiltinFunc { name: String, func: BuiltinFunc },
}

impl MethodInfo {
    pub fn default() -> Self {
        MethodInfo::AttrReader {
            id: IdentId::from(0),
        }
    }

    pub fn as_iseq(&self, vm: &VM) -> Result<ISeqRef, RubyError> {
        if let MethodInfo::RubyFunc { iseq } = self {
            Ok(iseq.clone())
        } else {
            Err(vm.error_unimplemented("Methodref is illegal."))
        }
    }
    /*
    pub fn set_iseq_kind(&mut self, kind: ISeqKind) {
        if let MethodInfo::RubyFunc { iseq } = self {
            iseq.kind = kind;
        }
    }*/
}

impl std::fmt::Debug for MethodInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodInfo::RubyFunc { iseq } => write!(f, "RubyFunc {:?}", *iseq),
            MethodInfo::AttrReader { id } => write!(f, "AttrReader {:?}", id),
            MethodInfo::AttrWriter { id } => write!(f, "AttrWriter {:?}", id),
            MethodInfo::BuiltinFunc { name, .. } => write!(f, "BuiltinFunc {:?}", name),
        }
    }
}

pub type ISeqRef = Ref<ISeqInfo>;

#[derive(Debug, Clone)]
pub struct ISeqInfo {
    pub method: MethodRef,
    pub params: ISeqParams,
    pub iseq: ISeq,
    pub lvar: LvarCollector,
    pub lvars: usize,
    /// The Class where this method was described.
    /// This field is set to None when IseqInfo was created by Codegen.
    /// Later, when the VM execute Inst::DEF_METHOD or DEF_SMETHOD,
    /// Set to Some() in class definition context, or None in the top level.
    pub class_defined: Option<ClassListRef>,
    pub iseq_sourcemap: Vec<(ISeqPos, Loc)>,
    pub source_info: SourceInfoRef,
    pub kind: ISeqKind,
}

#[derive(Debug, Clone)]
pub struct ISeqParams {
    pub req_params: usize,
    pub opt_params: usize,
    pub rest_param: bool,
    pub post_params: usize,
    pub block_param: bool,
    pub min_params: usize,
    pub max_params: usize,
    pub param_ident: Vec<IdentId>,
    pub keyword_params: HashMap<IdentId, LvarId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassList {
    /// The outer class of `class`.
    pub outer: Option<ClassListRef>,
    /// The class where ISeqInfo was described.
    pub class: Value,
}

pub type ClassListRef = Ref<ClassList>;

impl ClassList {
    pub fn new(outer: Option<ClassListRef>, class: Value) -> Self {
        ClassList { outer, class }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ISeqKind {
    Other,
    Method(IdentId),  // Method or Lambda
    Block(MethodRef), // Block or Proc
}

impl ISeqInfo {
    pub fn new(
        method: MethodRef,
        req_params: usize,
        opt_params: usize,
        rest_param: bool,
        post_params: usize,
        block_param: bool,
        min_params: usize,
        max_params: usize,
        param_ident: Vec<IdentId>,
        keyword_params: HashMap<IdentId, LvarId>,
        iseq: ISeq,
        lvar: LvarCollector,
        iseq_sourcemap: Vec<(ISeqPos, Loc)>,
        source_info: SourceInfoRef,
        kind: ISeqKind,
    ) -> Self {
        let lvars = lvar.len();
        ISeqInfo {
            method,
            params: ISeqParams {
                req_params,
                opt_params,
                rest_param,
                post_params,
                block_param,
                min_params,
                max_params,
                param_ident,
                keyword_params,
            },
            iseq,
            lvar,
            lvars,
            class_defined: None,
            iseq_sourcemap,
            source_info,
            kind,
        }
    }

    pub fn default(method: MethodRef) -> Self {
        ISeqInfo::new(
            method,
            0,
            0,
            false,
            0,
            false,
            0,
            0,
            vec![],
            std::collections::HashMap::new(),
            vec![],
            LvarCollector::new(),
            vec![],
            SourceInfoRef::empty(),
            ISeqKind::Method(IdentId::from(0)),
        )
    }
}

#[derive(Debug, Clone)]
pub struct GlobalMethodTable {
    table: Vec<MethodInfo>,
    method_id: u32,
}

impl GlobalMethodTable {
    pub fn new() -> Self {
        GlobalMethodTable {
            table: vec![MethodInfo::AttrReader {
                id: IdentId::from(1),
            }],
            method_id: 1,
        }
    }

    pub fn add_method(&mut self, info: MethodInfo) -> MethodRef {
        let new_method = MethodRef(self.method_id);
        self.method_id += 1;
        self.table.push(info);
        new_method
    }

    pub fn new_method(&mut self) -> MethodRef {
        let new_method = MethodRef(self.method_id);
        self.method_id += 1;
        self.table.push(MethodInfo::default());
        new_method
    }

    pub fn set_method(&mut self, method: MethodRef, info: MethodInfo) {
        self.table[method.0 as usize] = info;
    }

    pub fn get_method(&self, method: MethodRef) -> &MethodInfo {
        &self.table[method.0 as usize]
    }

    pub fn get_mut_method(&mut self, method: MethodRef) -> &mut MethodInfo {
        &mut self.table[method.0 as usize]
    }
}

//----------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodObjInfo {
    pub name: IdentId,
    pub receiver: Value,
    pub method: MethodRef,
}

impl MethodObjInfo {
    pub fn new(name: IdentId, receiver: Value, method: MethodRef) -> Self {
        MethodObjInfo {
            name,
            receiver,
            method,
        }
    }
}

pub type MethodObjRef = Ref<MethodObjInfo>;

impl MethodObjRef {
    pub fn from(name: IdentId, receiver: Value, method: MethodRef) -> Self {
        MethodObjRef::new(MethodObjInfo::new(name, receiver, method))
    }
}
