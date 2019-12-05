use crate::vm::*;

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub iseq: ISeqRef,
    pub context: ContextRef,
}

impl ProcInfo {
    pub fn new(iseq: ISeqRef, context: ContextRef) -> Self {
        ProcInfo { iseq, context }
    }
}

pub type ProcRef = Ref<ProcInfo>;

impl ProcRef {
    pub fn from(iseq: ISeqRef, context: ContextRef) -> Self {
        ProcRef::new(ProcInfo::new(iseq, context))
    }
}

pub fn init_proc(globals: &mut Globals) -> ClassRef {
    let proc_id = globals.get_ident_id("Proc");
    let proc_class = ClassRef::from(proc_id, globals.object_class);
    globals.add_builtin_instance_method(proc_class, "call", procobj::proc_call);
    globals.add_builtin_class_method(proc_class, "new", procobj::proc_new);
    proc_class
}

// Class methods

fn proc_new(_vm: &mut VM, _receiver: PackedValue, _args: Vec<PackedValue>) -> VMResult {
    let procobj = PackedValue::nil();
    Ok(procobj)
}

// Instance methods

fn proc_call(vm: &mut VM, receiver: PackedValue, args: Vec<PackedValue>) -> VMResult {
    let pref = match receiver.as_proc() {
        Some(pref) => pref,
        None => return Err(vm.error_unimplemented("Expected Proc object.")),
    };
    vm.vm_run(
        pref.context.self_value,
        pref.context.iseq_ref,
        pref.context.outer,
        args,
    )?;
    let res = vm.exec_stack.pop().unwrap();
    Ok(res)
}
