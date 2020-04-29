use crate::*;

pub fn init_struct(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Struct");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "inspect", inspect);
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
            let s = vm.globals.get_ident_id(format!("Struct:{}", s));
            Some(s)
        }
    };

    let mut val = Value::class_from(&mut vm.globals, name, self_val);
    let class = val.as_class();
    vm.globals
        .add_builtin_instance_method(class, "initialize", initialize);

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
        vm.globals.get_ident_id("_members"),
        Value::array_from(&vm.globals, vec),
    );
    attr_accessor(vm, val, &attr_args)?;

    match args.block {
        Some(method) => {
            vm.class_push(val);
            let arg = Args::new1(None, val);
            vm.eval_method(method, val, &arg, true)?;
            vm.class_pop();
        }
        None => {}
    };
    Ok(val)
}

fn initialize(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    let class = self_val.get_class_object(&vm.globals);
    let members = class
        .get_var(vm.globals.get_ident_id("_members"))
        .unwrap()
        .as_array()
        .unwrap();
    if members.elements.len() < args.len() {
        return Err(vm.error_argument("Struct size differs."));
    };
    for (i, arg) in args.iter().enumerate() {
        let id = members.elements[i].as_symbol().unwrap();
        let var = format!("@{}", vm.globals.get_ident_name(id));
        self_val.set_var(vm.globals.get_ident_id(var), *arg);
        //eprintln!("{}:{}", var, vm.val_inspect(*arg));
    }
    Ok(Value::nil())
}

fn inspect(vm: &mut VM, _self_val: Value, _args: &Args) -> VMResult {
    /*
    let arg_string = {
        match eref.args.len() {
            0 => "".to_string(),
            1 => vm.val_inspect(eref.args[0]),
            _ => {
                let mut s = vm.val_inspect(eref.args[0]);
                for i in 1..eref.args.len() {
                    s = format!("{},{}", s, vm.val_inspect(eref.args[i]));
                }
                s
            }
        }
    };
    */
    let inspect = format!("#<struct: >");
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
        "#;
        assert_script(program);
    }
}
