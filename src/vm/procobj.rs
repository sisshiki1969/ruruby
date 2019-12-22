use crate::vm::*;

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub context: ContextRef,
}

impl ProcInfo {
    pub fn new(context: ContextRef) -> Self {
        ProcInfo { context }
    }
}

pub type ProcRef = Ref<ProcInfo>;

impl ProcRef {
    pub fn from(context: ContextRef) -> Self {
        ProcRef::new(ProcInfo::new(context))
    }
}

pub fn init_proc(globals: &mut Globals) -> ClassRef {
    let proc_id = globals.get_ident_id("Proc");
    let proc_class = ClassRef::from(proc_id, globals.object_class);
    globals.add_builtin_instance_method(proc_class, "call", proc_call);
    globals.add_builtin_class_method(proc_class, "new", proc_new);
    proc_class
}

// Class methods

fn proc_new(
    vm: &mut VM,
    _receiver: PackedValue,
    _args: Vec<PackedValue>,
    block: Option<ContextRef>,
) -> VMResult {
    let procobj = match block {
        Some(block) => PackedValue::procobj(&vm.globals, block),
        None => return Err(vm.error_type("Needs block.")),
    };
    Ok(procobj)
}

// Instance methods

fn proc_call(
    vm: &mut VM,
    receiver: PackedValue,
    args: Vec<PackedValue>,
    _block: Option<ContextRef>,
) -> VMResult {
    let pref = match receiver.as_proc() {
        Some(pref) => pref,
        None => return Err(vm.error_unimplemented("Expected Proc object.")),
    };
    vm.vm_run(
        pref.context.self_value,
        pref.context.iseq_ref,
        pref.context.outer,
        args,
        None,
    )?;
    let res = vm.exec_stack.pop().unwrap();
    Ok(res)
}
