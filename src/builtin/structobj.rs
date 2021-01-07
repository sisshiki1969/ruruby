use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let class = Value::class_under(globals.builtins.object);
    class.add_builtin_class_method("new", struct_new);
    class.get()
}

fn struct_new(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let self_val = Module::new(self_val);
    args.check_args_min(1)?;
    let mut i = 0;

    let mut class = Value::class_under(self_val);
    match args[0].as_string() {
        None => {}
        Some(s) => {
            match s.chars().nth(0) {
                Some(c) if c.is_ascii_uppercase() => {}
                _ => {
                    return Err(RubyError::name(format!(
                        "Identifier `{}` needs to be constant.",
                        s
                    )))
                }
            };
            i = 1;
            class.set_name(format!("Struct::{}", s))
        }
    };
    class.add_builtin_method_by_str("initialize", initialize);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_class_method("[]", builtin::class::new);
    class.add_builtin_class_method("new", builtin::class::new);

    let mut attr_args = Args::new(args.len() - i);
    let mut vec = vec![];
    for index in i..args.len() {
        let v = args[index];
        if v.as_symbol().is_none() {
            return Err(RubyError::typeerr(format!(
                "{:?} is not a symbol.",
                args[index]
            )));
        };
        vec.push(v);
        attr_args[index - i] = v;
    }
    class
        .get()
        .set_var_by_str("/members", Value::array_from(vec));
    builtin::module::set_attr_accessor(&mut vm.globals, class.get(), &attr_args)?;

    match &args.block {
        Block::None => {}
        method => {
            vm.class_push(class);
            let arg = Args::new1(class.get());
            let res = vm.eval_block_self(method, class.get(), &arg);
            vm.class_pop();
            res?;
        }
    };
    Ok(class.get())
}

fn initialize(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    let class = self_val.get_class();
    let name = class.get().get_var(IdentId::get_id("/members")).unwrap();
    let members = name.as_array().unwrap();
    if members.elements.len() < args.len() {
        return Err(RubyError::argument("Struct size differs."));
    };
    for (i, arg) in args.iter().enumerate() {
        let id = members.elements[i].as_symbol().unwrap();
        let var = format!("@{:?}", id);
        self_val.set_var_by_str(&var, *arg);
    }
    Ok(Value::nil())
}

use std::borrow::Cow;
fn inspect(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let mut inspect = format!("#<struct ");
    match self_val.get_class().op_name() {
        Some(name) => inspect += &name,
        None => {}
    };
    let name = match self_val
        .get_class()
        .get()
        .get_var(IdentId::get_id("/members"))
    {
        Some(name) => name,
        None => return Err(RubyError::internal("No /members.")),
    };
    //eprintln!("{:?}", name);
    let members = match name.as_array() {
        Some(aref) => aref,
        None => {
            return Err(RubyError::internal(format!(
                "Illegal _members value. {:?}",
                name
            )))
        }
    };

    for x in &members.elements {
        let id = IdentId::add_prefix(x.as_symbol().unwrap(), "@");
        let val = match self_val.get_var(id) {
            Some(v) => Cow::from(vm.val_inspect(v)?),
            None => Cow::from("nil"),
        };
        inspect = format!("{} {:?}={}", inspect, id, val);
    }
    inspect += ">";

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

    #[test]
    fn struct_inspect() {
        let program = r###"
        S = Struct.new(:a,:b)
        s = S.new(100,200)
        assert 100, s.a
        assert 200, s.b
        assert "#<struct S @a=100 @b=200>", s.inspect
        "###;
        assert_script(program);
    }
}
