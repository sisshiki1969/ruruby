use crate::*;
use std::cell::RefCell;
use std::time::Duration;

thread_local!(
    pub static METHODS: RefCell<MethodRepo> = RefCell::new(MethodRepo::new());
);

#[cfg(feature = "perf-method")]
thread_local!(
    pub static METHOD_PERF: RefCell<MethodPerf> = RefCell::new(MethodPerf::new());
);

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

impl Into<u32> for MethodId {
    fn into(self) -> u32 {
        self.0.get()
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

#[cfg(feature = "perf-method")]
pub struct MethodPerf {
    inline_hit: usize,
    inline_missed: usize,
    total: usize,
    missed: usize,
    timer: std::time::Instant,
    prev_time: Duration,
    prev_method: Option<MethodId>,
}

#[cfg(feature = "perf-method")]
impl MethodPerf {
    fn new() -> Self {
        Self {
            inline_hit: 0,
            inline_missed: 0,
            total: 0,
            missed: 0,
            timer: std::time::Instant::now(),
            prev_time: Duration::from_secs(0),
            prev_method: None,
        }
    }

    fn inc_inline_hit() {
        METHOD_PERF.with(|m| m.borrow_mut().inline_hit += 1);
    }

    fn inc_inline_missed() {
        METHOD_PERF.with(|m| m.borrow_mut().inline_missed += 1);
    }

    fn inc_total() {
        METHOD_PERF.with(|m| m.borrow_mut().total += 1);
    }

    fn inc_missed() {
        METHOD_PERF.with(|m| m.borrow_mut().missed += 1);
    }

    fn next(method: MethodId) {
        let (dur, prev_method) = METHOD_PERF.with(|m| {
            let elapsed = m.borrow().timer.elapsed();
            let prev = m.borrow().prev_time;
            let prev_method = m.borrow().prev_method;
            m.borrow_mut().prev_time = elapsed;
            m.borrow_mut().prev_method = Some(method);
            (elapsed - prev, prev_method)
        });
        let id = match prev_method {
            Some(it) => it,
            _ => return,
        };
        METHODS.with(|m| m.borrow_mut().counter[id.0.get() as usize].duration += dur);
    }

    pub fn clear_stats() {
        METHOD_PERF.with(|m| {
            m.borrow_mut().inline_hit = 0;
            m.borrow_mut().inline_missed = 0;
            m.borrow_mut().total = 0;
            m.borrow_mut().missed = 0;
        });
    }

    pub fn print_stats() {
        METHOD_PERF.with(|m| {
            let perf = m.borrow();
            eprintln!("+-------------------------------------------+");
            eprintln!("| Method cache stats:                       |");
            eprintln!("+-------------------------------------------+");
            eprintln!("  hit inline cache    : {:>10}", perf.inline_hit);
            eprintln!("  missed inline cache : {:>10}", perf.inline_missed);
            eprintln!("  hit global cache    : {:>10}", perf.total - perf.missed);
            eprintln!("  missed              : {:>10}", perf.missed);
        });
    }
}

#[derive(Debug, Clone)]
struct MethodRepoCounter {
    count: usize,
    duration: Duration,
}

impl std::default::Default for MethodRepoCounter {
    fn default() -> Self {
        Self {
            count: 0,
            duration: Duration::from_secs(0),
        }
    }
}

pub struct MethodRepo {
    table: Vec<MethodInfo>,
    counter: Vec<MethodRepoCounter>,
    class_version: u32,
    i_cache: InlineCache,
    m_cache: MethodCache,
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
                    class: "Enumerator".to_string(),
                }, // METHOD_ENUM
            ],
            counter: vec![
                MethodRepoCounter::default(),
                MethodRepoCounter::default(),
                MethodRepoCounter::default(),
            ],
            class_version: 0,
            i_cache: InlineCache::new(),
            m_cache: MethodCache::new(),
        }
    }

    pub fn add(info: MethodInfo) -> MethodId {
        METHODS.with(|m| {
            let m = &mut m.borrow_mut();
            m.table.push(info);
            m.counter.push(MethodRepoCounter::default());
            MethodId::new((m.table.len() - 1) as u32)
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

    pub fn inc_class_version() {
        METHODS.with(|m| m.borrow_mut().class_version += 1)
    }

    pub fn add_inline_cache_entry() -> u32 {
        METHODS.with(|m| m.borrow_mut().i_cache.add_entry())
    }

    pub fn find_method_inline_cache(
        id: u32,
        rec_class: Module,
        method_name: IdentId,
    ) -> Option<MethodId> {
        METHODS.with(|m| {
            let mut repo = m.borrow_mut();
            let class_version = repo.class_version;
            match repo.i_cache.get_entry(id) {
                Some(InlineCacheEntry {
                    version,
                    class,
                    method,
                }) if *version == class_version && class.id() == rec_class.id() => {
                    #[cfg(feature = "perf-method")]
                    MethodPerf::inc_inline_hit();
                    return Some(*method);
                }
                _ => {}
            };
            #[cfg(feature = "perf-method")]
            MethodPerf::inc_inline_missed();
            if let Some(method_id) = repo
                .m_cache
                .get_method(class_version, rec_class, method_name)
            {
                repo.i_cache.update_entry(
                    id,
                    InlineCacheEntry::new(class_version, rec_class, method_id),
                );
                Some(method_id)
            } else {
                None
            }
        })
    }

    /// Search global method cache with receiver class and method name.
    ///
    /// If the method was not found, return None.
    pub fn find_method(rec_class: Module, method_id: IdentId) -> Option<MethodId> {
        METHODS.with(|m| {
            let mut repo = m.borrow_mut();
            let class_version = repo.class_version;
            repo.m_cache.get_method(class_version, rec_class, method_id)
        })
    }

    /// Search global method cache with receiver object and method class_name.
    ///
    /// If the method was not found, return None.
    pub fn find_method_from_receiver(receiver: Value, method_id: IdentId) -> Option<MethodId> {
        let rec_class = receiver.get_class_for_method();
        Self::find_method(rec_class, method_id)
    }

    pub fn mark(alloc: &mut Allocator) {
        let keys: Vec<Module> =
            METHODS.with(|m| m.borrow().m_cache.cache.keys().map(|(v, _)| *v).collect());
        keys.iter().for_each(|m| m.mark(alloc));
    }
}

#[cfg(feature = "perf-method")]
impl MethodRepo {
    pub fn inc_counter(id: MethodId) {
        MethodPerf::next(id);
        METHODS.with(|m| m.borrow_mut().counter[id.0.get() as usize].count += 1);
    }

    pub fn clear_stats() {
        METHODS.with(|m| {
            m.borrow_mut()
                .counter
                .iter_mut()
                .for_each(|c| *c = MethodRepoCounter::default());
        });
        MethodPerf::clear_stats();
    }

    pub fn print_stats() {
        eprintln!(
            "+-----------------------------------------------------------------------------------------------------+"
        );
        eprintln!(
            "| Method call stats:                                                                                  |"
        );
        eprintln!(
            "+-----------------------------------------------------------------------------------------------------+"
        );
        eprintln!(
            "  MethodId({:>5}) {:>12} {:>15}   info",
            "id", "exec count", "time"
        );
        METHODS.with(|m| {
            let mut v: Vec<_> = m
                .borrow()
                .counter
                .iter()
                .enumerate()
                .map(|(id, counter)| (id, counter.clone()))
                .collect();
            v.sort_by_key(|x| x.1.duration);
            for (id, count) in v.iter().rev() {
                if count.count > 0 {
                    let time = format!("{:?}", count.duration);
                    eprintln!(
                        "  MethodId({:>5}) {:>12} {:>15}   {:?}",
                        id,
                        count.count,
                        time,
                        m.borrow().table[*id]
                    );
                }
            }
        });
    }
}

pub type BuiltinFunc = fn(vm: &mut VM, self_val: Value, args: &Args) -> VMResult;

pub type MethodTable = FxIndexMap<IdentId, MethodId>;

pub static METHOD_ENUM: MethodId = MethodId(unsafe { std::num::NonZeroU32::new_unchecked(2) });

#[derive(Clone)]
pub enum MethodInfo {
    RubyFunc {
        iseq: ISeqRef,
    },
    AttrReader {
        id: IdentId,
    },
    AttrWriter {
        id: IdentId,
    },
    BuiltinFunc {
        name: IdentId,
        func: BuiltinFunc,
        class: String,
    },
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
            MethodInfo::RubyFunc { iseq } => write!(f, "RubyFunc {:?}", **iseq),
            MethodInfo::AttrReader { id } => write!(f, "AttrReader {:?}", id),
            MethodInfo::AttrWriter { id } => write!(f, "AttrWriter {:?}", id),
            MethodInfo::BuiltinFunc { name, class, .. } => {
                write!(f, r##"BuiltinFunc {}#{:?}"##, class, name)
            }
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

///---------------------------------------------------------------------------------------------------
///
///  Inline method cache
///
///  This module supports inline method cache which is embedded in the instruction sequence directly.
///
///---------------------------------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct InlineCache {
    table: Vec<Option<InlineCacheEntry>>,
    id: u32,
}

#[derive(Debug, Clone)]
pub struct InlineCacheEntry {
    pub version: u32,
    pub class: Module,
    pub method: MethodId,
}

impl InlineCacheEntry {
    fn new(version: u32, class: Module, method: MethodId) -> Self {
        InlineCacheEntry {
            version,
            class,
            method,
        }
    }
}

impl InlineCache {
    fn new() -> Self {
        InlineCache {
            table: vec![],
            id: 0,
        }
    }
    fn add_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(None);
        self.id - 1
    }

    fn get_entry(&self, id: u32) -> &Option<InlineCacheEntry> {
        &self.table[id as usize]
    }

    fn update_entry(&mut self, id: u32, entry: InlineCacheEntry) {
        self.table[id as usize] = Some(entry);
    }
}

///---------------------------------------------------------------------------------------------------
///
/// Global method cache
///
/// This module supports global method cache.
///
///---------------------------------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct MethodCache {
    cache: FxHashMap<(Module, IdentId), MethodCacheEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodCacheEntry {
    pub method: MethodId,
    pub version: u32,
}

impl MethodCache {
    fn new() -> Self {
        MethodCache {
            cache: FxHashMap::default(),
        }
    }

    fn add_entry(&mut self, class: Module, id: IdentId, version: u32, method: MethodId) {
        self.cache
            .insert((class, id), MethodCacheEntry { method, version });
    }

    fn get_entry(&self, class: Module, id: IdentId) -> Option<&MethodCacheEntry> {
        self.cache.get(&(class, id))
    }

    /// Get corresponding instance method(MethodId) for the class object `class` and `method`.
    ///
    /// If an entry for `class` and `method` exists in global method cache and the entry is not outdated,
    /// return MethodId of the entry.
    /// If not, search `method` by scanning a class chain.
    /// `class` must be a Class.
    fn get_method(
        &mut self,
        class_version: u32,
        rec_class: Module,
        method: IdentId,
    ) -> Option<MethodId> {
        #[cfg(feature = "perf-method")]
        {
            MethodPerf::inc_total();
        }
        if let Some(MethodCacheEntry { version, method }) = self.get_entry(rec_class, method) {
            if *version == class_version {
                return Some(*method);
            }
        };
        #[cfg(feature = "perf-method")]
        {
            MethodPerf::inc_missed();
        }
        match rec_class.search_method(method) {
            Some(methodref) => {
                self.add_entry(rec_class, method, class_version, methodref);
                Some(methodref)
            }
            None => None,
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

#[derive(Clone, Copy, PartialEq)]
pub enum ISeqKind {
    Other,                   // eval or unnamed method
    Method(Option<IdentId>), // method or lambda
    Class(IdentId),          // class definition
    Block,                   // block or proc
}

impl Default for ISeqKind {
    fn default() -> Self {
        ISeqKind::Other
    }
}

impl std::fmt::Debug for ISeqKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other => write!(f, "Other"),
            Self::Class(id) => write!(f, r##"Class["{:?}"]"##, id),
            Self::Method(None) => write!(f, "Method[unnamed]"),
            Self::Method(Some(id)) => write!(f, r##"Method["{:?}"]"##, id),
            Self::Block => write!(f, "Block"),
        }
    }
}

pub type ISeqRef = Ref<ISeqInfo>;

#[derive(Clone, Default)]
pub struct ISeqInfo {
    pub method: MethodId,
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
}

impl std::fmt::Debug for ISeqInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let class_name = match self.class_defined.last() {
            Some(class) => format!("{:?}#", class),
            None => "".to_string(),
        };
        let func_name = match self.kind {
            ISeqKind::Block => "Block".to_string(),
            ISeqKind::Method(id) => match id {
                Some(id) => format!("Method: {}{:?}", class_name, id),
                None => format!("Method: {}<unnamed>", class_name),
            },
            ISeqKind::Class(id) => format!("Class: {:?}", id),
            ISeqKind::Other => "Other".to_string(),
        };
        write!(f, "{} opt:{:?}", func_name, self.opt_flag)
    }
}

impl ISeqInfo {
    pub fn new(
        method: MethodId,
        params: ISeqParams,
        iseq: ISeq,
        lvar: LvarCollector,
        exception_table: Vec<ExceptionEntry>,
        iseq_sourcemap: Vec<(ISeqPos, Loc)>,
        source_info: SourceInfoRef,
        kind: ISeqKind,
    ) -> Self {
        let lvars = lvar.len();
        let opt_flag = params.is_opt();
        ISeqInfo {
            method,
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
        }
    }

    pub fn is_block(&self) -> bool {
        match self.kind {
            ISeqKind::Block => true,
            _ => false,
        }
    }

    pub fn is_method(&self) -> bool {
        !self.is_block()
    }

    pub fn is_classdef(&self) -> bool {
        match self.kind {
            ISeqKind::Class(_) => true,
            _ => false,
        }
    }
}

//----------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodObjInfo {
    pub name: IdentId,
    pub receiver: Option<Value>,
    pub method: MethodId,
    pub owner: Module,
}

impl MethodObjInfo {
    pub fn new(name: IdentId, receiver: Value, method: MethodId, owner: Module) -> Self {
        MethodObjInfo {
            name,
            receiver: Some(receiver),
            method,
            owner,
        }
    }

    pub fn new_unbound(name: IdentId, method: MethodId, owner: Module) -> Self {
        MethodObjInfo {
            name,
            receiver: None,
            method,
            owner,
        }
    }
}

impl GC for MethodObjInfo {
    fn mark(&self, alloc: &mut Allocator) {
        if let Some(rec) = self.receiver {
            rec.mark(alloc);
        }
    }
}
