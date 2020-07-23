use crate::*;

pub type BuiltinFunc = fn(vm: &mut VM, self_val: Value, args: &Args) -> VMResult;

pub type MethodTable = FxHashMap<IdentId, MethodRef>;

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
    pub fn print(&self, globals: &Globals) {
        let info = globals.get_method_info(*self);
        let iseq = if let MethodInfo::RubyFunc { iseq } = info {
            *iseq
        } else {
            panic!("CodeGen: Illegal methodref.")
        };
        println!("-----------------------------------------");
        let name = IdentId::get_ident_name(iseq.name);
        println!("{} {:?} opt_flag:{:?}", name, *self, iseq.opt_flag);
        print!("local var: ");
        for (k, v) in iseq.lvar.table() {
            print!("{}:{} ", v.as_u32(), IdentId::get_ident_name(*k));
        }
        println!("");
        println!("block: {:?}", iseq.lvar.block());
        println!("-----------------------------------------");
        /*let mut pc = 0;
        while pc < iseq.iseq.len() {
            println!("  {:05x} {}", pc, Inst::inst_info(globals, iseq, pc));
            pc += Inst::inst_size(iseq.iseq[pc]);
        }*/
    }
}

#[derive(Clone)]
pub enum MethodInfo {
    RubyFunc { iseq: ISeqRef },
    AttrReader { id: IdentId },
    AttrWriter { id: IdentId },
    BuiltinFunc { name: String, func: BuiltinFunc },
}

impl GC for MethodInfo {
    fn mark(&self, alloc: &mut Allocator) {
        match self {
            MethodInfo::RubyFunc { iseq } => match iseq.class_defined {
                Some(list) => list.mark(alloc),
                None => return,
            },
            _ => return,
        };
    }
}

impl MethodInfo {
    pub fn default() -> Self {
        MethodInfo::AttrReader {
            id: IdentId::from(0),
        }
    }

    pub fn as_iseq(&self, vm: &VM) -> Result<ISeqRef, RubyError> {
        if let MethodInfo::RubyFunc { iseq } = self {
            Ok(*iseq)
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
    pub method: MethodRef,
    pub name: Option<IdentId>,
    pub params: ISeqParams,
    pub iseq: ISeq,
    pub lvar: LvarCollector,
    pub lvars: usize,
    /// This flag is set when the following conditions are met.
    /// 1) Not a block.
    /// 2) Has no optional/post/rest/block/keyword parameters.
    pub opt_flag: bool,
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
    pub param_ident: Vec<IdentId>,
    pub keyword_params: FxHashMap<IdentId, LvarId>,
}

impl ISeqParams {
    pub fn is_opt(&self) -> bool {
        self.opt_params == 0
            && !self.rest_param
            && self.post_params == 0
            && !self.block_param
            && self.keyword_params.is_empty()
    }
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

impl GC for ClassList {
    fn mark(&self, alloc: &mut Allocator) {
        self.class.mark(alloc);
        match self.outer {
            Some(list) => list.mark(alloc),
            None => return,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ISeqKind {
    Other,           // eval or unnamed method
    Method(IdentId), // method or lambda
    Block,           // block or proc
}

impl ISeqInfo {
    pub fn new(
        method: MethodRef,
        name: Option<IdentId>,
        req_params: usize,
        opt_params: usize,
        rest_param: bool,
        post_params: usize,
        block_param: bool,
        param_ident: Vec<IdentId>,
        keyword_params: FxHashMap<IdentId, LvarId>,
        iseq: ISeq,
        lvar: LvarCollector,
        iseq_sourcemap: Vec<(ISeqPos, Loc)>,
        source_info: SourceInfoRef,
        kind: ISeqKind,
    ) -> Self {
        let lvars = lvar.len();
        let params = ISeqParams {
            req_params,
            opt_params,
            rest_param,
            post_params,
            block_param,
            param_ident,
            keyword_params,
        };
        let opt_flag = match kind {
            ISeqKind::Block => false,
            _ => params.is_opt(),
        };
        ISeqInfo {
            method,
            name,
            params,
            iseq,
            lvar,
            lvars,
            opt_flag,
            class_defined: None,
            iseq_sourcemap,
            source_info,
            kind,
        }
    }

    pub fn default(method: MethodRef) -> Self {
        ISeqInfo::new(
            method,
            None,
            0,
            0,
            false,
            0,
            false,
            vec![],
            FxHashMap::default(),
            vec![],
            LvarCollector::new(),
            vec![],
            SourceInfoRef::empty(),
            ISeqKind::Method(IdentId::from(0)),
        )
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

#[derive(Debug, Clone)]
pub struct GlobalMethodTable {
    table: Vec<MethodInfo>,
    method_id: u32,
}

impl GlobalMethodTable {
    pub fn new() -> Self {
        GlobalMethodTable {
            table: vec![MethodInfo::BuiltinFunc {
                func: enumerator_iterate,
                name: "/enum".to_string(),
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

impl GC for GlobalMethodTable {
    fn mark(&self, alloc: &mut Allocator) {
        self.table.iter().for_each(|m| m.mark(alloc));
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

impl GC for MethodObjInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.receiver.mark(alloc);
    }
}
