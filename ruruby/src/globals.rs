use crate::*;
use fancy_regex::Regex;
use std::path::{Path, PathBuf};
use std::rc::Rc;
mod constants;
mod method;
use constants::*;
pub use method::*;
#[cfg(feature = "perf-method")]
mod method_perf;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub const_values: ConstantValues,
    global_var: ValueTable,
    const_cache: ConstCache,
    pub case_dispatch: CaseDispatchMap,
    pub case_dispatch2: CaseDispatchMap2,

    pub main_fiber: Option<VMRef>,
    pub instant: std::time::Instant,
    pub const_version: u32,
    pub main_object: Value,
    pub regexp_cache: FxHashMap<String, Rc<Regex>>,
    source_files: Vec<PathBuf>,
    #[cfg(feature = "perf")]
    pub perf: Perf,
    pub startup_flag: bool,
    /// register for error handling
    pub val: Value,
    pub fiber_result: VMResult,
    pub methods: MethodRepo,
}

pub type GlobalsRef = Ref<Globals>;

impl GC<RValue> for Globals {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        self.const_values.mark(alloc);
        self.main_object.mark(alloc);
        self.global_var.values().for_each(|v| v.mark(alloc));
        for t in &self.case_dispatch.table {
            t.keys().for_each(|k| k.mark(alloc));
        }
        if let Some(vm) = self.main_fiber {
            vm.mark(alloc);
        }
        self.val.mark(alloc);
    }
}

impl GCRoot<RValue> for Globals {
    #[inline(always)]
    fn startup_flag(&self) -> bool {
        self.startup_flag
    }
}

impl Globals {
    pub(crate) fn new() -> Self {
        use builtin::*;
        EssentialClass::init();
        BuiltinClass::init();
        let object = BuiltinClass::object();
        let main_object = Value::ordinary_object(object);
        let mut globals = Globals {
            const_values: ConstantValues::new(),
            global_var: FxHashMap::default(),
            const_cache: ConstCache::new(),
            main_fiber: None,
            instant: std::time::Instant::now(),
            const_version: 0,
            main_object,
            case_dispatch: CaseDispatchMap::new(),
            case_dispatch2: CaseDispatchMap2::new(),
            regexp_cache: FxHashMap::default(),
            source_files: vec![],
            #[cfg(feature = "perf")]
            perf: Perf::new(),
            startup_flag: false,
            val: Value::nil(),
            fiber_result: Ok(Value::nil()),
            methods: MethodRepo::new(),
        };

        BuiltinClass::initialize(&mut globals);

        io::init(&mut globals);
        file::init(&mut globals);

        main_object.set_var_by_str("/name", Value::string("main"));

        let mut env_map = HashInfo::new(FxIndexMap::default());
        std::env::vars()
            .for_each(|(var, val)| env_map.insert(Value::string(var), Value::string(val)));
        #[cfg(windows)]
        if let None = env_map.get(&Value::string("HOME")) {
            let home_drive = env_map.get(&Value::string("HOMEDRIVE"));
            let home_path = env_map.get(&Value::string("HOMEPATH"));
            let user_profile = env_map.get(&Value::string("USERPROFILE"));
            let home = if home_drive.is_some() && home_drive.is_some() {
                home_drive.unwrap().as_string().unwrap().to_string()
                    + home_path.unwrap().as_string().unwrap()
            } else if let Some(up) = user_profile {
                up.as_string().unwrap().to_string()
            } else {
                "".to_string()
            };
            env_map.insert(
                Value::string("HOME"),
                Value::string(home.replace('\\', "/")),
            );
        };

        let env = Value::hash_from(env_map);
        globals.set_toplevel_constant("ENV", env);
        globals.set_global_var_by_str("$/", Value::string("\n"));
        globals
    }

    pub(crate) fn add_source_file(&mut self, file_path: &Path) -> Option<usize> {
        if self.source_files.contains(&file_path.to_path_buf()) {
            None
        } else {
            let i = self.source_files.len();
            self.source_files.push(file_path.to_owned());
            Some(i)
        }
    }

    pub(crate) fn from_exception(&self, err: &RubyError) -> Option<Value> {
        let err = err.clone();
        let val = match &err.kind {
            RubyErrorKind::Exception => return None,
            RubyErrorKind::ParseErr(_) => {
                let err_class = self.get_toplevel_constant("SyntaxError").into_module();
                Value::exception(err_class, err)
            }
            RubyErrorKind::RuntimeErr { kind, .. } => match kind {
                RuntimeErrKind::Type => {
                    let err_class = self.get_toplevel_constant("TypeError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::Argument => {
                    let err_class = self.get_toplevel_constant("ArgumentError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::NoMethod => {
                    let err_class = self.get_toplevel_constant("NoMethodError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::Runtime => {
                    let err_class = self.get_toplevel_constant("RuntimeError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::LoadError => {
                    let err_class = self.get_toplevel_constant("LoadError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::StopIteration => {
                    let err_class = self.get_toplevel_constant("StopIteration").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::Name => {
                    let err_class = self.get_toplevel_constant("NameError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::ZeroDivision => {
                    let err_class = self
                        .get_toplevel_constant("ZeroDivisionError")
                        .into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::Range => {
                    let err_class = self.get_toplevel_constant("RangeError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::Index => {
                    let err_class = self.get_toplevel_constant("IndexError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::Regexp => {
                    let err_class = self.get_toplevel_constant("RegexpError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::Fiber => {
                    let err_class = self.get_toplevel_constant("FiberError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::LocalJump => {
                    let err_class = self.get_toplevel_constant("LocalJumpError").into_module();
                    Value::exception(err_class, err)
                }
                RuntimeErrKind::DomainError => {
                    let math = self.get_toplevel_constant("Math");
                    let err_class = math
                        .into_module()
                        .get_const_noautoload(IdentId::get_id("DomainError"))
                        .unwrap()
                        .into_module();
                    Value::exception(err_class, err)
                }
            },
            RubyErrorKind::MethodReturn | RubyErrorKind::BlockReturn => {
                let err_class = self.get_toplevel_constant("LocalJumpError").into_module();
                Value::exception(err_class, err)
            }
            _ => {
                let standard = BuiltinClass::standard();
                Value::exception(standard, err)
            }
        };
        Some(val)
    }

    pub fn show_err(&self, err: &RubyError) {
        match self.from_exception(err) {
            Some(ex) => match ex.if_exception() {
                Some(err) => eprintln!("{:?}", err),
                None => unreachable!(),
            },
            None => eprint!("None"),
        }
    }

    #[cfg(feature = "gc-debug")]
    pub fn print_mark(&self) {
        ALLOC.with(|m| m.borrow_mut().print_mark());
    }

    #[cfg(any(feature = "emit-iseq", feature = "trace"))]
    pub(crate) fn inst_info(&self, iseq_ref: ISeqRef, pc: ISeqPos) -> String {
        fn imm_i32(iseq: &ISeq, pc: ISeqPos) -> String {
            format!(
                "{} {}",
                Inst::inst_name(iseq[pc]),
                iseq.read32(pc + 1) as i32
            )
        }
        let iseq = &iseq_ref.iseq;
        match iseq[pc] {
            Inst::ADDI
            | Inst::SUBI
            | Inst::EQI
            | Inst::NEI
            | Inst::GTI
            | Inst::GEI
            | Inst::LTI
            | Inst::LEI
            | Inst::GET_IDX_I
            | Inst::SET_IDX_I => imm_i32(iseq, pc),
            Inst::PUSH_VAL => format!("PUSH_VAL {:?}", Value::from(iseq.read64(pc + 1))),

            Inst::JMP
            | Inst::JMP_BACK
            | Inst::JMP_F
            | Inst::JMP_T
            | Inst::JMP_F_EQ
            | Inst::JMP_F_NE
            | Inst::JMP_F_GT
            | Inst::JMP_F_GE
            | Inst::JMP_F_LT
            | Inst::JMP_F_LE => format!(
                "{} {:>05x}",
                Inst::inst_name(iseq[pc]),
                (pc + 5 + iseq.read_disp(pc + 1)).into_usize()
            ),

            Inst::JMP_F_EQI
            | Inst::JMP_F_NEI
            | Inst::JMP_F_GTI
            | Inst::JMP_F_GEI
            | Inst::JMP_F_LTI
            | Inst::JMP_F_LEI => format!(
                "{} {} {:>05x}",
                Inst::inst_name(iseq[pc]),
                iseq.read32(pc + 1) as i32,
                (pc + 9 + iseq.read_disp(pc + 5)).into_usize()
            ),

            Inst::OPT_CASE => format!(
                "OPT_CASE {:>05}",
                (pc + 13 + iseq.read_disp(pc + 9)).into_usize(),
            ),
            Inst::SET_LOCAL | Inst::GET_LOCAL => {
                let id = iseq.read32(pc + 1);
                let ident_name = iseq_ref.lvar.get_name(id.into());
                format!("{} '{}'", Inst::inst_name(iseq[pc]), ident_name,)
            }
            Inst::SET_DYNLOCAL | Inst::GET_DYNLOCAL => {
                let frame = iseq.read32(pc + 5);
                let id = iseq.read32(pc + 1);
                format!(
                    "{} outer:{} LvarId:{}",
                    Inst::inst_name(iseq[pc]),
                    frame,
                    id
                )
            }
            Inst::CHECK_LOCAL => {
                let frame = iseq.read32(pc + 5);
                let id = iseq.read32(pc + 1) as usize;
                let ident_id = iseq_ref.lvar.get_name(id.into());
                format!("CHECK_LOCAL '{:?}' outer:{}", ident_id, frame)
            }
            Inst::GET_CONST
            | Inst::GET_CONST_TOP
            | Inst::SET_CONST
            | Inst::CHECK_CONST
            | Inst::CHECK_METHOD
            | Inst::GET_SCOPE
            | Inst::GET_IVAR
            | Inst::SET_IVAR
            | Inst::CHECK_IVAR
            | Inst::GET_CVAR
            | Inst::SET_CVAR
            | Inst::GET_GVAR
            | Inst::SET_GVAR
            | Inst::CHECK_GVAR => format!(
                "{} '{}'",
                Inst::inst_name(iseq[pc]),
                iseq.ident_name(pc + 1)
            ),
            Inst::GET_SVAR | Inst::SET_SVAR => {
                format!("{}({})", Inst::inst_name(iseq[pc]), iseq.read32(pc + 1))
            }
            Inst::SEND | Inst::SEND_SELF => format!(
                "{} '{}' args:{} block:{} flag:{:?}",
                Inst::inst_name(iseq[pc]),
                iseq.ident_name(pc + 1),
                iseq.read16(pc + 5),
                iseq.read_block(pc + 8),
                iseq.read_argflag(pc + 7),
            ),
            Inst::OPT_SEND | Inst::OPT_SEND_SELF | Inst::OPT_SEND_N | Inst::OPT_SEND_SELF_N => {
                format!(
                    "{} '{}' args:{} block:{}",
                    Inst::inst_name(iseq[pc]),
                    iseq.ident_name(pc + 1),
                    iseq.read16(pc + 5),
                    iseq.read_block(pc + 7),
                )
            }
            Inst::SUPER => format!(
                "{} args:{} block:{} {}",
                Inst::inst_name(iseq[pc]),
                iseq.read16(pc + 1),
                iseq.read_block(pc + 3),
                if iseq.read8(pc + 7) == 1 {
                    "NO_ARGS"
                } else {
                    ""
                },
            ),

            Inst::CREATE_ARRAY
            | Inst::CREATE_PROC
            | Inst::CREATE_HASH
            | Inst::DUP
            | Inst::TAKE
            | Inst::SINKN
            | Inst::TOPN
            | Inst::CONCAT_STRING
            | Inst::YIELD
            | Inst::RESCUE => format!(
                "{} {} items",
                Inst::inst_name(iseq[pc]),
                iseq.read32(pc + 1)
            ),
            Inst::CONST_VAL => {
                let id = iseq.read32(pc + 1);
                format!("CONST_VAL {:?}", self.const_values.get(id as usize))
            }
            Inst::DEF_CLASS => format!(
                "DEF_CLASS {} '{}' method:{}",
                if iseq.read8(pc + 1) == 1 {
                    "module"
                } else {
                    "class"
                },
                iseq.ident_name(pc + 2),
                iseq.read32(pc + 6)
            ),
            Inst::DEF_SCLASS => "DEF_SCLASS".to_string(),
            Inst::DEF_METHOD => format!("DEF_METHOD '{}'", iseq.ident_name(pc + 1)),
            Inst::DEF_SMETHOD => format!("DEF_SMETHOD '{}'", iseq.ident_name(pc + 1)),
            _ => Inst::inst_name(iseq[pc]),
        }
    }
}

///
/// Contant value
///
/// A table which holds constant values.
///
#[derive(Debug, Clone)]
pub struct ConstantValues {
    table: Vec<Value>,
}

impl ConstantValues {
    pub(crate) fn new() -> Self {
        Self { table: vec![] }
    }

    pub(crate) fn insert(&mut self, val: Value) -> usize {
        let id = self.table.len();
        self.table.push(val);
        id
    }

    pub(crate) fn get(&self, id: usize) -> Value {
        self.table[id].shallow_dup()
    }

    #[cfg(not(tarpaulin_include))]
    #[cfg(features = "emit-iseq")]
    pub(crate) fn dump(&self) {
        for (i, val) in self.table.iter().enumerate() {
            eprintln!("{}:{:?}", i, val);
        }
    }
}

impl GC<RValue> for ConstantValues {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        self.table.iter().for_each(|v| v.mark(alloc));
    }
}

///
/// Case dispatch map.
///
/// This module supports optimization for case-when syntax when all of the when-conditions were integer literals.
///
#[derive(Debug, Clone)]
pub struct CaseDispatchMap {
    table: Vec<FxHashMap<HashKey, ISeqDisp>>,
    id: u32,
}

impl CaseDispatchMap {
    fn new() -> Self {
        CaseDispatchMap {
            table: vec![],
            id: 0,
        }
    }

    pub(crate) fn new_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(FxHashMap::default());
        self.id - 1
    }

    pub(crate) fn get_entry(&self, id: u32) -> &FxHashMap<HashKey, ISeqDisp> {
        &self.table[id as usize]
    }

    pub(crate) fn get_mut_entry(&mut self, id: u32) -> &mut FxHashMap<HashKey, ISeqDisp> {
        &mut self.table[id as usize]
    }
}

///
/// Case dispatch map-2.
///
/// This module supports optimization for case-when syntax when all of the when-conditions were integer literals.
///
#[derive(Debug, Clone)]
pub struct CaseDispatchMap2 {
    table: Vec<(i64, i64, Vec<ISeqDisp>)>, //(min, max, map)
}

impl CaseDispatchMap2 {
    fn new() -> Self {
        Self { table: vec![] }
    }

    pub(crate) fn new_entry(&mut self) -> u32 {
        let len = self.table.len();
        self.table.push((0, 0, vec![]));
        len as u32
    }

    pub(crate) fn get_entry(&self, id: u32) -> &(i64, i64, Vec<ISeqDisp>) {
        &self.table[id as usize]
    }

    pub(crate) fn get_mut_entry(&mut self, id: u32) -> &mut (i64, i64, Vec<ISeqDisp>) {
        &mut self.table[id as usize]
    }
}
