use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under_object();
    globals.set_toplevel_constant("Struct", class);
    class.add_builtin_class_method(globals, "new", struct_new);
    class.into()
}

fn struct_new(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    let self_val = self_val.into_module();
    args.check_args_min(1)?;
    let mut i = 0;

    let mut class = Module::class_under(self_val);
    let arg0 = vm[0];
    match arg0.as_string() {
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
    class.add_builtin_method_by_str(&mut vm.globals, "initialize", initialize);
    class.add_builtin_method_by_str(&mut vm.globals, "inspect", inspect);
    class.add_builtin_class_method(&mut vm.globals, "[]", builtin::class::new);
    class.add_builtin_class_method(&mut vm.globals, "new", builtin::class::new);

    let mut attr_args = Args::new(args.len() - i);
    let mut vec = vec![];
    for index in i..args.len() {
        let v = vm[index];
        if v.as_symbol().is_none() {
            return Err(RubyError::typeerr(format!(
                "{:?} is not a symbol.",
                vm[index]
            )));
        };
        vec.push(v);
        attr_args[index - i] = v;
    }
    class.set_var_by_str("/members", Value::array_from(vec));
    builtin::module::set_attr_accessor(&mut vm.globals, class, &attr_args)?;

    match &args.block {
        None => {}
        Some(block) => {
            let arg = Args::new1(class.into());
            let res = vm.eval_block_self(block, class, &arg);
            res?;
        }
    };
    Ok(class.into())
}

fn initialize(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    let class = vm.globals.get_class(self_val);
    let name = class.get_var(IdentId::get_id("/members")).unwrap();
    let members = name.into_array();
    if members.len() < args.len() {
        return Err(RubyError::argument("Struct size differs."));
    };
    for (i, arg) in vm.args().iter().enumerate() {
        let id = members[i].as_symbol().unwrap();
        let var = format!("@{:?}", id);
        self_val.set_var_by_str(&var, *arg);
    }
    Ok(Value::nil())
}

use std::borrow::Cow;
fn inspect(vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    let mut inspect = format!("#<struct ");
    match vm.globals.get_class(self_val).op_name() {
        Some(name) => inspect += &name,
        None => {}
    };
    let name = match vm
        .globals
        .get_class(self_val)
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

    for x in &**members {
        let id = x.as_symbol().unwrap().add_prefix("@");
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
    use crate::tests::*;

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
