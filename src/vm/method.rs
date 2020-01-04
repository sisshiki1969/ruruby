use crate::vm::*;

pub type BuiltinFunc =
    fn(vm: &mut VM, receiver: PackedValue, args: VecArray, block: Option<MethodRef>) -> VMResult;

pub type MethodTable = HashMap<IdentId, MethodRef>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodRef(usize);

impl std::hash::Hash for MethodRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Into<u32> for MethodRef {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl From<u32> for MethodRef {
    fn from(id: u32) -> Self {
        MethodRef(id as usize)
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
    pub fn as_iseq(&self, vm: &VM) -> Result<ISeqRef, RubyError> {
        if let MethodInfo::RubyFunc { iseq } = self {
            Ok(iseq.clone())
        } else {
            Err(vm.error_unimplemented("Methodref is illegal."))
        }
    }
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
    pub params: Vec<LvarId>,
    pub min_params: usize,
    pub max_params: usize,
    pub iseq: ISeq,
    pub lvar: LvarCollector,
    pub lvars: usize,
    pub iseq_sourcemap: Vec<(ISeqPos, Loc)>,
    pub source_info: SourceInfoRef,
}

impl ISeqInfo {
    pub fn new(
        params: Vec<LvarId>,
        min_params: usize,
        max_params: usize,
        iseq: ISeq,
        lvar: LvarCollector,
        iseq_sourcemap: Vec<(ISeqPos, Loc)>,
        source_info: SourceInfoRef,
    ) -> Self {
        let lvars = lvar.len();
        ISeqInfo {
            params,
            min_params,
            max_params,
            iseq,
            lvar,
            lvars,
            iseq_sourcemap,
            source_info,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlobalMethodTable {
    table: Vec<MethodInfo>,
    method_id: usize,
}

impl GlobalMethodTable {
    pub fn new() -> Self {
        GlobalMethodTable {
            table: vec![MethodInfo::AttrReader {
                id: IdentId::from(0),
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

    pub fn get_method(&self, method: MethodRef) -> &MethodInfo {
        &self.table[method.0]
    }

    pub fn get_mut_method(&mut self, method: MethodRef) -> &mut MethodInfo {
        &mut self.table[method.0]
    }
}
