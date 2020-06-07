use crate::*;

pub fn init_struct(globals: &mut Globals) -> Value {
    let id = IdentId::get_ident_id("Struct");
    let class = ClassRef::from(id, globals.builtins.object);
    let class = Value::class(globals, class);
    globals.add_builtin_class_method(class, "new", struct_new);
    class
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
            let s = IdentId::get_ident_id(format!("Struct:{}", s));
            Some(s)
        }
    };

    let mut val = Value::class_from(&mut vm.globals, name, self_val);
    let class = val.as_class();
    vm.globals
        .add_builtin_instance_method(class, "initialize", initialize);
    vm.globals
        .add_builtin_instance_method(class, "inspect", inspect);
    vm.globals
        .add_builtin_class_method(val, "[]", builtin::class::new);
    vm.globals
        .add_builtin_class_method(val, "new", builtin::class::new);

    let mut attr_args = Args::new(args.len() - i);
    let mut vec = vec![];
    for index in i..args.len() {
        let v = args[index];
        if v.as_symbol().is_none() {
            let n = vm.val_inspect(v);
            return Err(vm.error_type(format!("{} is not a symbol.", n)));
        };
        vec.push(v);
        attr_args[index - i] = v;
    }
    val.set_var(
        IdentId::get_ident_id("_members"),
        Value::array_from(&vm.globals, vec),
    );
    builtin::module::attr_accessor(vm, val, &attr_args)?;

    match args.block {
        Some(method) => {
            vm.class_push(val);
            let arg = Args::new1(val);
            vm.eval_method(method, val, Some(vm.context()), &arg)?;
            vm.class_pop();
        }
        None => {}
    };
    Ok(val)
}

fn initialize(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    let class = self_val.get_class_object(&vm.globals);
    let members = class
        .get_var(IdentId::get_ident_id("_members"))
        .unwrap()
        .as_array()
        .unwrap();
    if members.elements.len() < args.len() {
        return Err(vm.error_argument("Struct size differs."));
    };
    for (i, arg) in args.iter().enumerate() {
        let id = members.elements[i].as_symbol().unwrap();
        let var = format!("@{}", IdentId::get_ident_name(id));
        self_val.set_var(IdentId::get_ident_id(var), *arg);
    }
    Ok(Value::nil())
}

fn inspect(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let members = match self_val
        .get_class_object(&vm.globals)
        .get_var(IdentId::get_ident_id("_members"))
    {
        Some(v) => match v.as_array() {
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
            let name = format!("@{}", IdentId::get_ident_name(id));
            IdentId::get_ident_id(name)
        })
        .collect();
    let mut attr_str = String::new();
    for id in attrs {
        let val = match self_val.get_var(id) {
            Some(v) => vm.val_inspect(v),
            None => "<>".to_string(),
        };
        let name = IdentId::get_ident_name(id);

        attr_str = format!("{} {}={}", attr_str, name, val);
    }
    let class_name = match self_val.get_class_object(&vm.globals).as_class().name {
        Some(id) => IdentId::get_ident_name(id),
        None => "".to_string(),
    };
    let inspect = format!("#<struct: {}{}>", class_name, attr_str);
    Ok(Value::string(&vm.globals, inspect))
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
