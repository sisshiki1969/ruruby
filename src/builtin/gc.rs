use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let id = IdentId::get_id("GC");
    let class = ClassRef::from(id, BuiltinClass::object());
    let mut class_val = Value::module(class);
    //class.add_builtin_instance_method( "to_s", to_s);
    class_val.add_builtin_class_method("count", count);
    class_val.add_builtin_class_method("enable", enable);
    class_val.add_builtin_class_method("disable", disable);
    class_val.add_builtin_class_method("start", start);
    class_val.add_builtin_class_method("stat", stat);
    class_val.add_builtin_class_method("print_mark", print_mark);
    class_val
}

fn count(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 0)?;
    let count = vm.globals.allocator.count();
    Ok(Value::integer(count as i64))
}

fn enable(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 0)?;
    let last_state = vm.globals.gc_enabled;
    vm.globals.gc_enabled = true;
    Ok(Value::bool(last_state))
}

fn disable(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 0)?;
    let last_state = vm.globals.gc_enabled;
    vm.globals.gc_enabled = false;
    Ok(Value::bool(last_state))
}

fn start(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.globals.gc();
    Ok(Value::nil())
}

fn stat(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    let mut hash = FxHashMap::default();
    let alloc = vm.globals.allocator;
    macro_rules! stat_insert {
        ( $($symbol:ident, $num:expr);* ) => {$(
            let id = IdentId::get_id(stringify!($symbol));
            hash.insert(HashKey(Value::symbol(id)), Value::integer($num as i64));
        )*};
    }
    stat_insert!(count, alloc.count());
    stat_insert!(heap_allocated_pages, alloc.pages_len());
    stat_insert!(heap_free_slots, alloc.free_count());
    stat_insert!(heap_live_slots, alloc.live_count());
    stat_insert!(total_allocated_objects, alloc.total_allocated());
    let res = Value::hash_from_map(hash);
    Ok(res)
}

fn print_mark(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    let mut alloc = vm.globals.allocator;
    alloc.gc_mark_only(&vm.globals);
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
