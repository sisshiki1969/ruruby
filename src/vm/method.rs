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
    pub req_params: usize,
    pub opt_params: usize,
    pub rest_param: bool,
    pub post_params: usize,
    pub block_param: bool,
    pub min_params: usize,
    pub max_params: usize,
    pub param_ident: Vec<IdentId>,
    pub keyword_params: HashMap<IdentId, LvarId>,
    pub iseq: ISeq,
    pub lvar: LvarCollector,
    pub lvars: usize,
    pub iseq_sourcemap: Vec<(ISeqPos, Loc)>,
    pub source_info: SourceInfoRef,
}

impl ISeqInfo {
    pub fn new(
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
    ) -> Self {
        let lvars = lvar.len();
        ISeqInfo {
            req_params,
            opt_params,
            rest_param,
            post_params,
            block_param,
            min_params,
            max_params,
            param_ident,
            keyword_params,
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

    pub fn get_method(&self, method: MethodRef) -> &MethodInfo {
        &self.table[method.0]
    }

    pub fn get_mut_method(&mut self, method: MethodRef) -> &mut MethodInfo {
        &mut self.table[method.0]
    }
}

//----------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MethodObjInfo {
    pub name: IdentId,
    pub receiver: PackedValue,
    pub method: MethodRef,
}

impl MethodObjInfo {
    pub fn new(name: IdentId, receiver: PackedValue, method: MethodRef) -> Self {
        MethodObjInfo {
            name,
            receiver,
            method,
        }
    }
}

pub type MethodObjRef = Ref<MethodObjInfo>;

impl MethodObjRef {
    pub fn from(name: IdentId, receiver: PackedValue, method: MethodRef) -> Self {
        MethodObjRef::new(MethodObjInfo::new(name, receiver, method))
    }
}

pub fn init_method(globals: &mut Globals) -> PackedValue {
    let proc_id = globals.get_ident_id("Method");
    let class = ClassRef::from(proc_id, globals.object);
    globals.add_builtin_instance_method(class, "call", method_call);
    PackedValue::class(globals, class)
}

fn method_call(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    block: Option<MethodRef>,
) -> VMResult {
    let method = match receiver.as_method() {
        Some(method) => method,
        None => return Err(vm.error_unimplemented("Expected Method object.")),
    };
    vm.eval_send(method.method, method.receiver, args, None, block)?;
    let res = vm.stack_pop();
    Ok(res)
}
