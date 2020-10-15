use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Struct");
    let class = ClassInfo::from(id, BuiltinClass::object());
    let mut class_val = Value::class(class);
    class_val.add_builtin_class_method("new", struct_new);
    class_val
}

fn struct_new(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_min(args.len(), 1)?;
    let mut i = 0;
    let name = match args[0].as_string() {
        None => None,
        Some(s) => {
            match s.chars().nth(0) {
                Some(c) if c.is_ascii_uppercase() => {}
                _ => return Err(vm.error_name(format!("Identifier `{}` needs to be constant.", s))),
            };
            i = 1;
            let s = IdentId::get_id(&format!("Struct:{}", s));
            Some(s)
        }
    };

    let mut class_val = Value::class_from(name, self_val);
    let class = class_val.as_mut_class();
    class.add_builtin_method_by_str("initialize", initialize);
    class.add_builtin_method_by_str("inspect", inspect);
    class_val.add_builtin_class_method("[]", builtin::class::new);
    class_val.add_builtin_class_method("new", builtin::class::new);

    let mut attr_args = Args::new(args.len() - i);
    let mut vec = vec![];
    for index in i..args.len() {
        let v = args[index];
        if v.as_symbol().is_none() {
            let n = vm.val_inspect(v)?;
            return Err(vm.error_type(format!("{} is not a symbol.", n)));
        };
        vec.push(v);
        attr_args[index - i] = v;
    }
    class_val.set_var_by_str("_members", Value::array_from(vec));
    builtin::module::attr_accessor(vm, class_val, &attr_args)?;

    match args.block {
        Some(method) => {
            vm.class_push(class_val);
            let arg = Args::new1(class_val);
            vm.eval_method(method, class_val, Some(vm.current_context()), &arg)?;
            vm.class_pop();
        }
        None => {}
    };
    Ok(class_val)
}

fn initialize(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    let class = self_val.get_class();
    let name = class.get_var(IdentId::get_id("_members")).unwrap();
    let members = name.as_array().unwrap();
    if members.elements.len() < args.len() {
        return Err(vm.error_argument("Struct size differs."));
    };
    for (i, arg) in args.iter().enumerate() {
        let id = members.elements[i].as_symbol().unwrap();
        let var = format!("@{:?}", id);
        self_val.set_var_by_str(&var, *arg);
    }
    Ok(Value::nil())
}

fn inspect(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let mut name = self_val.get_class().get_var(IdentId::get_id("_members"));
    let members = match name {
        Some(ref mut v) => match v.as_array() {
            Some(aref) => aref,
            None => return Err(vm.error_internal("Illegal _members value.")),
        },
        None => return Err(vm.error_internal("No _members.")),
    };
    let attrs: Vec<IdentId> = members
        .elements
        .iter()
        .map(|x| {
            let id = x.as_symbol().unwrap();
            let name = format!("@{:?}", id);
            IdentId::get_id(&name)
        })
        .collect();
    let mut attr_str = String::new();
    for id in attrs {
        let val = match self_val.get_var(id) {
            Some(v) => vm.val_inspect(v)?,
            None => "<>".to_string(),
        };
        attr_str = format!("{} {:?}={}", attr_str, id, val);
    }
    let class_name = match self_val.get_class().as_class().name {
        Some(id) => IdentId::get_ident_name(id),
        None => "".to_string(),
    };
    let inspect = format!("#<struct: {}{}>", class_name, attr_str);
    Ok(Value::string(inspect))
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn struct_test() {
        let program = r#"
        Customer = Struct.new(:name, :address) do
            def greeting
                "Hello #{name}!"
            end
        end
        assert "Hello Dave!", Customer.new("Dave", "123 Main").greeting
        assert "Hello Gave!", Customer["Gave", "456 Sub"].greeting
        "#;
        assert_script(program);
    }
}
