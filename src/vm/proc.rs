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
    globals.add_builtin_instance_method(proc_class, "call", proc::proc_call);
    globals.add_builtin_class_method(proc_class, "new", proc::proc_new);
    proc_class
}

// Class methods

pub fn proc_new(_vm: &mut VM, _receiver: PackedValue, _args: Vec<PackedValue>) -> VMResult {
    let procobj = PackedValue::nil();
    Ok(procobj)
}

// Instance methods

pub fn proc_call(vm: &mut VM, receiver: PackedValue, args: Vec<PackedValue>) -> VMResult {
    let pref = match receiver.as_proc() {
        Some(pref) => pref,
        None => return Err(vm.error_unimplemented("Expected Proc object.")),
    };
    let mut context = pref.context;
    let arg_len = args.len();
    for (i, id) in pref.iseq.params.clone().iter().enumerate() {
        context.lvar_scope[id.as_usize()] = if i < arg_len {
            args[i]
        } else {
            PackedValue::nil()
        };
    }
    vm.context_stack.last_mut().unwrap().pc = vm.pc;
    vm.vm_run(context)?;
    let res = vm.exec_stack.pop().unwrap();
    Ok(res)
}
