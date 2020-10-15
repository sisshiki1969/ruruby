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
    let mut proc_class = ClassInfo::from(proc_id, BuiltinClass::object());
    proc_class.add_builtin_method_by_str("to_s", inspect);
    proc_class.add_builtin_method_by_str("inspect", inspect);
    proc_class.add_builtin_method_by_str("call", proc_call);
    proc_class.add_builtin_method_by_str("[]", proc_call);

    let mut class_val = Value::class(proc_class);
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

fn inspect(_: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let pref = self_val.as_proc().unwrap();
    let is_proc = pref.context.kind == ISeqKind::Block;
    let s = if is_proc {
        format!("#<Proc:0x{:016x}>", pref.context.id())
    } else {
        format!("#<Proc:0x{:016x}> (lambda)", pref.context.id())
    };
    Ok(Value::string(s))
}

fn proc_call(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let pref = self_val.as_proc().unwrap();
    let context = Context::from_args(
        vm,
        pref.context.self_value,
        pref.context.iseq_ref.unwrap(),
        args,
        pref.context.outer,
        vm.latest_context(),
    )?;
    let res = vm.run_context(ContextRef::from_local(&context))?;
    Ok(res)
}

#[cfg(test)]
mod test {
    use crate::test::*;

    #[test]
    fn proc() {
        let program = "
        @foo = 42
        p = Proc.new { @foo}
        l = lambda { @foo }
        assert(42, p[])
        assert(42, l[])
        ";
        assert_script(program);
    }
}
