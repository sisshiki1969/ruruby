#[cfg(feature = "perf-method")]
use super::method_perf::*;
use crate::*;

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

    pub(crate) fn as_iseq(&self, globals: &Globals) -> ISeqRef {
        globals.methods[*self].as_iseq()
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

#[derive(Debug, Clone)]
pub struct MethodRepo {
    table: Vec<MethodInfo>,
    class_version: u32,
    i_cache: InlineCache,
    m_cache: MethodCache,
    #[cfg(feature = "perf-method")]
    counter: Vec<MethodRepoCounter>,
    #[cfg(feature = "perf-method")]
    perf: MethodPerf,
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
    pub(crate) fn new() -> Self {
        Self {
            table: vec![
                MethodInfo::Void, // dummy
                MethodInfo::Void, // default
                MethodInfo::BuiltinFunc {
                    func: enumerator_iterate,
                    name: IdentId::_ENUM_FUNC,
                    class: IdentId::get_id("Enumerator"),
                }, // METHOD_ENUM
            ],
            #[cfg(feature = "perf-method")]
            counter: vec![
                MethodRepoCounter::default(),
                MethodRepoCounter::default(),
                MethodRepoCounter::default(),
            ],
            class_version: 0,
            i_cache: InlineCache::new(),
            m_cache: MethodCache::new(),
            #[cfg(feature = "perf-method")]
            perf: MethodPerf::new(),
        }
    }

    pub(crate) fn add(&mut self, info: MethodInfo) -> MethodId {
        self.table.push(info);
        #[cfg(feature = "perf-method")]
        self.counter.push(MethodRepoCounter::default());
        MethodId::new((self.table.len() - 1) as u32)
    }

    pub(crate) fn update(&mut self, id: MethodId, info: MethodInfo) {
        self[id] = info;
    }

    pub(crate) fn get(&self, id: MethodId) -> &MethodInfo {
        &self[id]
    }

    pub(crate) fn inc_class_version(&mut self) {
        self.class_version += 1;
    }

    pub(crate) fn add_inline_cache_entry(&mut self) -> u32 {
        self.i_cache.add_entry()
    }

    pub(crate) fn find_method_inline_cache(
        &mut self,
        id: u32,
        rec_class: Module,
        method_name: IdentId,
    ) -> Option<MethodId> {
        let class_version = self.class_version;
        match self.i_cache.get_entry(id) {
            Some(InlineCacheEntry {
                version,
                class,
                method,
            }) if *version == class_version && class.id() == rec_class.id() => {
                #[cfg(feature = "perf-method")]
                self.perf.inc_inline_hit();
                return Some(*method);
            }
            _ => {}
        };
        #[cfg(feature = "perf-method")]
        self.perf.inc_inline_missed();
        if let Some(method_id) = self.get_method_from_mcache(class_version, rec_class, method_name)
        {
            self.i_cache.update_entry(
                id,
                InlineCacheEntry::new(class_version, rec_class, method_id),
            );
            Some(method_id)
        } else {
            None
        }
    }

    /// Search global method cache with receiver class and method name.
    ///
    /// If the method was not found, return None.
    pub(crate) fn find_method(
        &mut self,
        rec_class: Module,
        method_id: IdentId,
    ) -> Option<MethodId> {
        let class_version = self.class_version;
        self.get_method_from_mcache(class_version, rec_class, method_id)
    }

    /// Search global method cache with receiver object and method class_name.
    ///
    /// If the method was not found, return None.
    pub(crate) fn find_method_from_receiver(
        &mut self,
        receiver: Value,
        method_id: IdentId,
    ) -> Option<MethodId> {
        let rec_class = receiver.get_class_for_method();
        self.find_method(rec_class, method_id)
    }

    /// Get corresponding instance method(MethodId) for the class object `class` and `method`.
    ///
    /// If an entry for `class` and `method` exists in global method cache and the entry is not outdated,
    /// return MethodId of the entry.
    /// If not, search `method` by scanning a class chain.
    /// `class` must be a Class.
    fn get_method_from_mcache(
        &mut self,
        class_version: u32,
        rec_class: Module,
        method: IdentId,
    ) -> Option<MethodId> {
        #[cfg(feature = "perf-method")]
        {
            self.perf.inc_total();
        }
        if let Some(MethodCacheEntry { version, method }) =
            self.m_cache.get_entry(rec_class, method)
        {
            if *version == class_version {
                return Some(*method);
            }
        };
        #[cfg(feature = "perf-method")]
        {
            self.perf.inc_missed();
        }
        match rec_class.search_method(method) {
            Some(methodref) => {
                self.m_cache
                    .add_entry(rec_class, method, class_version, methodref);
                Some(methodref)
            }
            None => None,
        }
    }
}

#[cfg(feature = "perf-method")]
impl MethodRepo {
    pub(crate) fn inc_counter(&mut self, id: MethodId) {
        let (dur, prev_method) = self.perf.next(id);
        match prev_method {
            Some(id) => self.counter[id.0.get() as usize].duration_inc(dur),
            _ => {}
        };
        self.counter[id.0.get() as usize].count_inc();
    }

    pub(crate) fn clear_stats(&mut self) {
        self.counter
            .iter_mut()
            .for_each(|c| *c = MethodRepoCounter::default());
        self.perf.clear_stats();
    }

    pub fn print_stats(&self) {
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
        let mut v: Vec<_> = self
            .counter
            .iter()
            .enumerate()
            .map(|(id, counter)| (id, counter.clone()))
            .collect();
        v.sort_by_key(|x| x.1.duration());
        for (id, count) in v.iter().rev() {
            if count.count() > 0 {
                let time = format!("{:?}", count.duration());
                eprintln!(
                    "  MethodId({:>5}) {:>12} {:>15}   {:?}",
                    id,
                    count.count(),
                    time,
                    self.table[*id]
                );
            }
        }
    }

    pub fn print_cache_stats(&self) {
        self.perf.print_cache_stats();
    }
}

pub type BuiltinFunc = fn(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult;

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
        class: IdentId,
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
                write!(f, r##"BuiltinFunc {:?}#{:?}"##, class, name)
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
    pub(crate) fn as_iseq(&self) -> ISeqRef {
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
}

//----------------------------------------------------------------------------------

#[derive(Default, Debug, Clone)]
pub struct ISeqParams {
    pub param_ident: Vec<IdentId>,
    pub req: usize,
    pub opt: usize,
    /// A flag for rest parameter.
    /// * Some(true): exists and bound to a param
    /// * Some(false): exists but to be discarded
    /// * None: not exists.
    pub rest: Option<bool>,
    pub post: usize,
    pub block: bool,
    pub keyword: FxHashMap<IdentId, LvarId>,
    pub kwrest: bool,
    /// A flag for argument delegation. e.g. f(...)
    pub delegate: Option<LvarId>,
}

impl ISeqParams {
    pub(crate) fn is_opt(&self) -> bool {
        self.opt == 0
            && self.rest.is_none()
            && self.post == 0
            && !self.block
            && self.keyword.is_empty()
            && !self.kwrest
            && self.delegate.is_none()
    }

    pub(crate) fn check_arity(&self, additional_kw: bool, args: &Args2) -> Result<(), RubyError> {
        let min = self.req + self.post;
        let kw = if additional_kw { 1 } else { 0 };
        if self.rest.is_some() {
            if min > kw {
                args.check_args_min(min - kw)?;
            }
        } else if self.delegate.is_none() {
            let len = args.len() + kw;
            if min > len || len > min + self.opt {
                return Err(RubyError::argument_wrong_range(len, min, min + self.opt));
            }
        } else {
            let len = args.len() + kw;
            if min > len {
                return Err(RubyError::argument(format!(
                    "Wrong number of arguments. (given {}, expected {}+)",
                    len, min
                )));
            }
        };
        Ok(())
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
    /// - Has no optional/post/rest/block/keyword/delegate parameters.
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
    pub loc: Loc,
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
    pub(crate) fn new(
        method: MethodId,
        params: ISeqParams,
        iseq: ISeq,
        lvar: LvarCollector,
        exception_table: Vec<ExceptionEntry>,
        iseq_sourcemap: Vec<(ISeqPos, Loc)>,
        source_info: SourceInfoRef,
        kind: ISeqKind,
        loc: Loc,
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
            loc,
        }
    }

    pub(crate) fn new_sym_to_proc(
        method: MethodId,
        iseq: ISeq,
        iseq_sourcemap: Vec<(ISeqPos, Loc)>,
        source_info: SourceInfoRef,
    ) -> Self {
        let id = IdentId::get_id("x");
        let lvar = LvarCollector::from(id);
        ISeqInfo {
            method,
            params: ISeqParams {
                param_ident: vec![id],
                req: 1,
                opt: 0,
                rest: None,
                post: 0,
                block: false,
                keyword: FxHashMap::default(),
                kwrest: false,
                delegate: None,
            },
            iseq,
            lvar,
            lvars: 1,
            exception_table: vec![],
            opt_flag: true,
            class_defined: vec![],
            iseq_sourcemap,
            source_info,
            kind: ISeqKind::Method(None),
            loc: Loc(0, 0),
        }
    }

    pub(crate) fn is_block(&self) -> bool {
        match self.kind {
            ISeqKind::Block => true,
            _ => false,
        }
    }

    pub(crate) fn is_method(&self) -> bool {
        !self.is_block()
    }

    pub(crate) fn is_classdef(&self) -> bool {
        match self.kind {
            ISeqKind::Class(_) => true,
            _ => false,
        }
    }
}

//----------------------------------------------------------------------------------

#[derive(Debug, Clone, Hash)]
pub struct MethodObjInfo {
    pub name: IdentId,
    pub receiver: Option<Value>,
    pub method: MethodId,
    pub owner: Module,
}

impl PartialEq for MethodObjInfo {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.method == other.method
            && match (self.receiver, other.receiver) {
                (Some(r1), Some(r2)) => r1.id() == r2.id(),
                _ => false,
            }
    }
}

impl MethodObjInfo {
    pub(crate) fn new(name: IdentId, receiver: Value, method: MethodId, owner: Module) -> Self {
        MethodObjInfo {
            name,
            receiver: Some(receiver),
            method,
            owner,
        }
    }

    pub(crate) fn new_unbound(name: IdentId, method: MethodId, owner: Module) -> Self {
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
