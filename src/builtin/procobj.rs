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

pub fn init_proc(globals: &mut Globals) -> PackedValue {
    let proc_id = globals.get_ident_id("Proc");
    let class = ClassRef::from(proc_id, globals.object);
    let obj = PackedValue::class(globals, class);
    globals.add_builtin_instance_method(class, "call", proc_call);
    globals.add_builtin_class_method(obj, "new", proc_new);
    obj
}

// Class methods

fn proc_new(vm: &mut VM, _args: &Args, block: Option<MethodRef>) -> VMResult {
    let procobj = match block {
        Some(block) => {
            let context = vm.create_context_from_method(block)?;
            PackedValue::procobj(&vm.globals, context)
        }
        None => return Err(vm.error_type("Needs block.")),
    };
    Ok(procobj)
}

// Instance methods

fn proc_call(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let pref = match args.self_value.as_proc() {
        Some(pref) => pref,
        None => return Err(vm.error_unimplemented("Expected Proc object.")),
    };
    let mut args = args.clone();
    args.self_value = pref.context.self_value;
    vm.vm_run(pref.context.iseq_ref, pref.context.outer, &args, None, None)?;
    let res = vm.stack_pop();
    Ok(res)
}
