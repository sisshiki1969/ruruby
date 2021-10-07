use crate::*;

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub self_val: Value,
    pub iseq: ISeqRef,
    pub outer: Option<HeapCtxRef>,
}

impl ProcInfo {
    pub fn new(self_val: Value, iseq: ISeqRef, outer: impl Into<Option<HeapCtxRef>>) -> Self {
        ProcInfo {
            self_val,
            iseq,
            outer: outer.into(),
        }
    }
}

impl GC for ProcInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.self_val.mark(alloc);
        if let Some(outer) = self.outer {
            outer.mark(alloc);
        }
    }
}

impl PartialEq for ProcInfo {
    fn eq(&self, other: &Self) -> bool {
        self.self_val.id() == other.self_val.id()
            && self.iseq == other.iseq
            && self.outer == other.outer
    }
}

pub fn init() -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Proc", class);
    class.add_builtin_method_by_str("to_s", inspect);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("call", proc_call);
    class.add_builtin_method_by_str("[]", proc_call);

    class.add_builtin_class_method("new", proc_new);
    class.into()
}

// Class methods

fn proc_new(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    let block = args.expect_block()?;
    let procobj = vm.create_proc(block);
    Ok(procobj)
}

// Instance methods

fn inspect(_: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    let pref = self_val.as_proc().unwrap();
    let s = if let ISeqKind::Block = pref.iseq.kind {
        format!("#<Proc:0x{:016x}>", self_val.id())
    } else {
        format!("#<Proc:0x{:016x}> (lambda)", self_val.id())
    };
    Ok(Value::string(s))
}

fn proc_call(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.eval_proc(self_val, &args.into(vm))
}

#[cfg(test)]
mod test {
    use crate::tests::*;

    #[test]
    fn proc() {
        let program = "
        foo = 42
        p = Proc.new { foo }
        p2 = proc { foo }
        l = lambda { foo }
        assert(42, p[])
        assert(42, p2[])
        assert(42, l[])
        ";
        assert_script(program);
    }
}
