use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let symbol_class = Module::class_under(globals.builtins.object);
    symbol_class.add_builtin_method_by_str("to_sym", to_sym);
    symbol_class.add_builtin_method_by_str("intern", to_sym);
    symbol_class.add_builtin_method_by_str("to_s", tos);
    symbol_class.add_builtin_method_by_str("id2name", tos);
    symbol_class.add_builtin_method_by_str("inspect", inspect);
    symbol_class.add_builtin_method_by_str("<=>", cmp);
    symbol_class.add_builtin_method_by_str("==", eq);
    symbol_class.into()
}

fn to_sym(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(self_val)
}

fn tos(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let s = IdentId::get_name(self_val.as_symbol().unwrap());
    Ok(Value::string(s))
}

fn inspect(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let s = format!(":{:?}", self_val.as_symbol().unwrap());
    Ok(Value::string(s))
}

fn cmp(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = IdentId::get_name(self_val.as_symbol().unwrap());
    let rhs = IdentId::get_name(match args[0].as_symbol() {
        Some(s) => s,
        None => return Ok(Value::nil()),
    });
    let ord = RString::string_cmp(&lhs.as_bytes(), &rhs.as_bytes());
    Ok(Value::integer(ord as i64))
}

fn eq(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_symbol().unwrap();
    let rhs = match args[0].as_symbol() {
        Some(id) => id,
        None => return Ok(Value::false_val()),
    };
    Ok(Value::bool(lhs == rhs))
}

#[cfg(test)]
mod test {
    use crate::test::*;
    #[test]
    fn symbol_test() {
        let program = r##"
        assert(:Ruby, :Ruby.to_sym)
        assert(:Ruby, :Ruby.intern)
        assert("Ruby", :Ruby.to_s)
        assert("Ruby", :Ruby.id2name)
        assert(":Ruby", :Ruby.inspect)
    "##;
        assert_script(program);
    }

    #[test]
    fn symbol_cmp() {
        let program = r##"
        assert(-1, :aaa <=> :xxx)
        assert(0, :aaa <=> :aaa)
        assert(1, :xxx <=> :aaa)
        assert(nil, :xxx <=> "xxx")
        assert(nil, :xxx <=> 333)

        assert(false, :aaa == :xxx)
        assert(true, :aaa == :aaa)
        assert(false, :xxx == :aaa)
        assert(false, :xxx == "xxx")
        assert(false, :xxx == 333)

        assert(false, :aaa.send(:"==", :xxx))
        assert(true, :aaa.send(:"==", :aaa))
        assert(false, :xxx.send(:"==", :aaa))
        assert(false, :xxx.send(:"==", "xxx"))
        assert(false, :xxx.send(:"==", 333))
    "##;
        assert_script(program);
    }
}
