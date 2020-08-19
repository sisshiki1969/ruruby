use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ProcInfo {
    pub context: ContextRef,
}

impl ProcInfo {
    pub fn new(context: ContextRef) -> Self {
        ProcInfo { context }
    }
}

pub fn init(_globals: &mut Globals) -> Value {
    let proc_id = IdentId::get_id("Proc");
    let mut proc_class = ClassRef::from(proc_id, BuiltinClass::object());
    let mut class_val = Value::class(proc_class);
    proc_class.add_builtin_instance_method("call", proc_call);
    class_val.add_builtin_class_method("new", proc_new);
    class_val
}

// Class methods

fn proc_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let method = vm.expect_block(args.block)?;
    let procobj = vm.create_proc(method)?;
    Ok(procobj)
}

// Instance methods

fn proc_call(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let pref = match self_val.as_proc() {
        Some(pref) => pref,
        None => return Err(vm.error_unimplemented("Expected Proc object.")),
    };
    let context = Context::from_args(
        vm,
        self_val,
        pref.context.iseq_ref.unwrap(),
        args,
        pref.context.outer,
        vm.latest_context(),
    )?;
    let res = vm.run_context(ContextRef::from_local(&context))?;
    Ok(res)
}
