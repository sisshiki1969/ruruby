use crate::vm::*;

pub type MethodTable = HashMap<IdentId, MethodInfo>;

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
    RubyFunc {
        params: Vec<LvarId>,
        iseq: ISeq,
        lvars: usize,
    },
    BuiltinFunc {
        name: String,
        func: BuiltinFunc,
    },
}

impl std::fmt::Debug for MethodInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodInfo::RubyFunc { params, .. } => write!(f, "RubyFunc {:?}", params),
            MethodInfo::BuiltinFunc { name, .. } => write!(f, "BuiltinFunc {:?}", name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlobalMethodTable {
    table: HashMap<MethodRef, MethodInfo>,
    method_id: usize,
}

impl GlobalMethodTable {
    pub fn new() -> Self {
        GlobalMethodTable {
            table: HashMap::new(),
            method_id: 0,
        }
    }
    pub fn add_method(&mut self, info: MethodInfo) -> MethodRef {
        let new_method = MethodRef(self.method_id);
        self.method_id += 1;
        self.table.insert(new_method, info);
        new_method
    }

    pub fn get_method(&self, method: MethodRef) -> &MethodInfo {
        self.table.get(&method).unwrap()
    }

    pub fn get_mut_method(&mut self, method: MethodRef) -> &mut MethodInfo {
        self.table.get_mut(&method).unwrap()
    }
}
