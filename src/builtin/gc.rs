use crate::*;

pub fn init() -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("GC", class);
    //class.add_builtin_instance_method( "to_s", to_s);
    class.add_builtin_class_method("count", count);
    class.add_builtin_class_method("enable", enable);
    class.add_builtin_class_method("disable", disable);
    class.add_builtin_class_method("start", start);
    class.add_builtin_class_method("stat", stat);
    class.add_builtin_class_method("print_mark", print_mark);
    class.into()
}

fn count(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let count = ALLOC.with(|m| m.borrow().count());
    Ok(Value::integer(count as i64))
}

fn enable(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let last_state = ALLOC.with(|m| {
        let enabled = m.borrow().gc_enabled;
        m.borrow_mut().gc_enabled = true;
        enabled
    });
    Ok(Value::bool(last_state))
}

fn disable(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let last_state = ALLOC.with(|m| {
        let enabled = m.borrow().gc_enabled;
        m.borrow_mut().gc_enabled = false;
        enabled
    });
    Ok(Value::bool(last_state))
}

fn start(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.globals.gc();
    Ok(Value::nil())
}

fn stat(_: &mut VM, _: Value, _: &Args) -> VMResult {
    let mut hash = FxHashMap::default();
    macro_rules! stat_insert {
        ( $($symbol:ident, $num:expr);* ) => {$(
            let id = IdentId::get_id(stringify!($symbol));
            hash.insert(HashKey(Value::symbol(id)), Value::integer($num as i64));
        )*};
    }
    stat_insert!(count, ALLOC.with(|m| m.borrow().count()));
    stat_insert!(heap_allocated_pages, ALLOC.with(|m| m.borrow().pages_len()));
    stat_insert!(heap_free_slots, ALLOC.with(|m| m.borrow().free_count()));
    stat_insert!(heap_live_slots, ALLOC.with(|m| m.borrow().live_count()));
    stat_insert!(
        total_allocated_objects,
        ALLOC.with(|m| m.borrow().total_allocated())
    );
    let res = Value::hash_from_map(hash);
    Ok(res)
}

fn print_mark(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    ALLOC.with(|m| m.borrow_mut().gc_mark_only(&vm.globals));
    Ok(Value::nil())
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn gc_module() {
        let program = r#"
            GC.count
            a = []
            100000.times do |x|
                a << x.to_s
            end
            GC.stat
            a = nil
            GC.start
            GC.print_mark
            GC.disable
            GC.enable
        "#;
        assert_script(program);
    }
}
