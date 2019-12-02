pub use crate::vm::*;

#[derive(Debug, Clone)]
pub struct Context {
    pub self_value: PackedValue,
    pub lvar_scope: Vec<PackedValue>,
    pub iseq_ref: ISeqRef,
    pub pc: usize,
    pub callmode: CallMode,
    pub outer: Option<ContextRef>,
}

pub type ContextRef = Ref<Context>;

#[derive(Debug, Clone, PartialEq)]
pub enum CallMode {
    Ordinary,
    FromNative,
    ClassDef,
}

impl Context {
    pub fn new(self_value: PackedValue, iseq_ref: ISeqRef, callmode: CallMode) -> Self {
        let lvar_num = iseq_ref.lvars;
        Context {
            self_value,
            lvar_scope: vec![PackedValue::nil(); lvar_num],
            iseq_ref,
            pc: 0,
            callmode,
            outer: None,
        }
    }
}

impl ContextRef {
    pub fn from(self_value: PackedValue, iseq_ref: ISeqRef, callmode: CallMode) -> Self {
        ContextRef::new(Context::new(self_value, iseq_ref, callmode))
    }
}
