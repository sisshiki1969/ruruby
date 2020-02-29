use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    class: Value,
    var_table: Box<ValueTable>,
    pub kind: ObjKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjKind {
    Ordinary,
    Class(ClassRef),
    Module(ClassRef),
    Range(RangeInfo),
    Array(ArrayRef),
    Splat(Value), // internal use only.
    Hash(HashRef),
    Proc(ProcRef),
    Regexp(RegexpRef),
    Method(MethodObjRef),
}

impl ObjectInfo {
    pub fn as_ref(&self) -> ObjectRef {
        Ref(unsafe {
            core::ptr::NonNull::new_unchecked(self as *const ObjectInfo as *mut ObjectInfo)
        })
    }

    pub fn new_bootstrap(classref: ClassRef) -> Self {
        ObjectInfo {
            class: Value::nil(), // dummy for boot strapping
            kind: ObjKind::Class(classref),
            var_table: Box::new(HashMap::new()),
        }
    }

    pub fn new_ordinary(class: Value) -> Self {
        ObjectInfo {
            class,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Ordinary,
        }
    }

    pub fn new_class(globals: &Globals, classref: ClassRef) -> Self {
        ObjectInfo {
            class: globals.builtins.class,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Class(classref),
        }
    }

    pub fn new_module(globals: &Globals, classref: ClassRef) -> Self {
        ObjectInfo {
            class: globals.builtins.module,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Module(classref),
        }
    }

    pub fn new_array(globals: &Globals, arrayref: ArrayRef) -> Self {
        ObjectInfo {
            class: globals.builtins.array,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Array(arrayref),
        }
    }

    pub fn new_splat(globals: &Globals, val: Value) -> Self {
        ObjectInfo {
            class: globals.builtins.array,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Splat(val),
        }
    }

    pub fn new_hash(globals: &Globals, hashref: HashRef) -> Self {
        ObjectInfo {
            class: globals.builtins.hash,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Hash(hashref),
        }
    }

    pub fn new_regexp(globals: &Globals, regexpref: RegexpRef) -> Self {
        ObjectInfo {
            class: globals.builtins.regexp,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Regexp(regexpref),
        }
    }

    pub fn new_range(globals: &Globals, info: RangeInfo) -> Self {
        ObjectInfo {
            class: globals.builtins.range,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Range(info),
        }
    }

    pub fn new_proc(globals: &Globals, procref: ProcRef) -> Self {
        ObjectInfo {
            class: globals.builtins.procobj,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Proc(procref),
        }
    }

    pub fn new_method(globals: &Globals, methodref: MethodObjRef) -> Self {
        ObjectInfo {
            class: globals.builtins.method,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Method(methodref),
        }
    }
}

pub type ObjectRef = Ref<ObjectInfo>;

impl ObjectRef {
    pub fn class(&self) -> Value {
        self.class
    }

    pub fn search_class(&self) -> Value {
        let mut class = self.class;
        loop {
            if class.as_class().is_singleton {
                class = class.as_object().class;
            } else {
                return class;
            }
        }
    }

    pub fn set_class(&mut self, class: Value) {
        self.class = class;
    }

    pub fn get_var(&self, id: IdentId) -> Option<Value> {
        self.var_table.get(&id).cloned()
    }

    pub fn get_mut_var(&mut self, id: IdentId) -> Option<&mut Value> {
        self.var_table.get_mut(&id)
    }

    pub fn set_var(&mut self, id: IdentId, val: Value) {
        self.var_table.insert(id, val);
    }

    pub fn var_table(&mut self) -> &mut ValueTable {
        &mut self.var_table
    }

    pub fn get_instance_method(&self, id: IdentId) -> Option<MethodRef> {
        self.search_class()
            .as_class()
            .method_table
            .get(&id)
            .cloned()
    }
}

pub fn init_object(globals: &mut Globals) {
    let object = globals.object_class;
    globals.add_builtin_instance_method(object, "class", class);
    globals.add_builtin_instance_method(object, "object_id", object_id);
    globals.add_builtin_instance_method(object, "singleton_class", singleton_class);
    globals.add_builtin_instance_method(object, "inspect", inspect);
    globals.add_builtin_instance_method(object, "eql?", eql);
    globals.add_builtin_instance_method(object, "to_i", toi);
    globals.add_builtin_instance_method(object, "instance_variable_set", instance_variable_set);
    globals.add_builtin_instance_method(object, "instance_variables", instance_variables);
    globals.add_builtin_instance_method(object, "floor", floor);
    globals.add_builtin_instance_method(object, "freeze", freeze);
    globals.add_builtin_instance_method(object, "super", super_);
    globals.add_builtin_instance_method(object, "equal?", equal);
    globals.add_builtin_instance_method(object, "send", send);
    globals.add_builtin_instance_method(object, "yield", object_yield);
    globals.add_builtin_instance_method(object, "eval", eval);
}

fn class(vm: &mut VM, args: &Args) -> VMResult {
    let class = args.self_value.get_class_object(&vm.globals);
    Ok(class)
}

fn object_id(_vm: &mut VM, args: &Args) -> VMResult {
    let id = args.self_value.id();
    Ok(Value::fixnum(id as i64))
}

fn singleton_class(vm: &mut VM, args: &Args) -> VMResult {
    vm.get_singleton_class(args.self_value)
}

fn inspect(vm: &mut VM, args: &Args) -> VMResult {
    let inspect = vm.val_pp(args.self_value);
    Ok(Value::string(inspect))
}

fn eql(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    Ok(Value::bool(args.self_value == args[0]))
}

fn toi(vm: &mut VM, args: &Args) -> VMResult {
    //vm.check_args_num(args.len(), 1, 1)?;
    let self_ = args.self_value;
    let num = if self_.is_packed_num() {
        if self_.is_packed_fixnum() {
            self_.as_packed_fixnum()
        } else {
            f64::trunc(self_.as_packed_flonum()) as i64
        }
    } else {
        return Err(vm.error_type("Must be a number."));
    };
    Ok(Value::fixnum(num))
}

fn instance_variable_set(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let name = args[0];
    let val = args[1];
    let var_id = match name.as_symbol() {
        Some(symbol) => symbol,
        None => match name.as_string() {
            Some(s) => vm.globals.get_ident_id(s),
            None => return Err(vm.error_type("1st arg must be Symbol or String.")),
        },
    };
    let mut self_obj = args.self_value.as_object();
    self_obj.set_var(var_id, val);
    Ok(val)
}

fn instance_variables(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut receiver = args.self_value.as_object();
    let res = receiver
        .var_table()
        .keys()
        .filter(|x| vm.globals.get_ident_name(**x).chars().nth(0) == Some('@'))
        .map(|x| Value::symbol(*x))
        .collect();
    Ok(Value::array_from(&vm.globals, res))
}

fn floor(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let rec = args.self_value;
    if rec.is_packed_fixnum() {
        Ok(rec)
    } else if rec.is_packed_num() {
        let res = rec.as_packed_flonum().floor() as i64;
        Ok(Value::fixnum(res))
    } else {
        Err(vm.error_type("Receiver must be Integer of Float."))
    }
}

fn freeze(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    Ok(args.self_value)
}

fn super_(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let context = vm.context();
    let iseq = context.iseq_ref.clone();
    if let ISeqKind::Method(m) = iseq.kind {
        let class = iseq.class_stack.as_ref().unwrap()[0];
        let method = match class.superclass() {
            Some(class) => vm.get_instance_method(class, m)?,
            None => {
                return Err(vm.error_nomethod(format!(
                    "no superclass method `{}' for {}.",
                    vm.globals.get_ident_name(m),
                    vm.val_pp(args.self_value),
                )))
            }
        };
        let param_num = iseq.param_ident.len();
        let mut args = Args::new0(context.self_value, None);
        for i in 0..param_num {
            args.push(context.get_lvar(LvarId::from_usize(i)));
        }
        vm.eval_send(method, &args, None)?;
        Ok(vm.stack_pop())
    } else {
        return Err(vm.error_nomethod("super called outside of method"));
    }
}

fn equal(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    Ok(Value::bool(args.self_value.id() == args[0].id()))
}

fn send(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 100)?;
    let receiver = args.self_value;
    let method_id = match args[0].as_symbol() {
        Some(symbol) => symbol,
        None => return Err(vm.error_argument("Must be a symbol.")),
    };
    let rec_class = receiver.get_class_object_for_method(&vm.globals);
    let method = vm.get_instance_method(rec_class, method_id)?;

    let mut new_args = Args::new(args.len() - 1);
    for i in 0..args.len() - 1 {
        new_args[i] = args[i + 1];
    }
    new_args.self_value = args.self_value;
    new_args.block = args.block;
    vm.eval_send(method, &new_args, None)?;
    let res = vm.stack_pop();
    Ok(res)
}

fn object_yield(vm: &mut VM, args: &Args) -> VMResult {
    let context = vm.context();
    let method = match context.block {
        Some(block) => block,
        None => return Err(vm.error_argument("Yield needs block.")),
    };
    vm.eval_send(method, &args, None)?;
    let res = vm.stack_pop();
    Ok(res)
}

fn eval(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let program = match args[0].as_string() {
        Some(s) => s,
        None => return Err(vm.error_argument("1st arg must be String.")),
    };
    let method = vm.parse_program_eval(std::path::PathBuf::from("eval"), program)?;
    let iseq = vm.get_iseq(method)?;
    let context = vm.context();
    let args = Args::new0(context.self_value, None);
    vm.vm_run(iseq, Some(context), &args, None)?;
    let res = vm.stack_pop();
    Ok(res)
}

#[cfg(test)]
mod test {
    use crate::test::*;

    #[test]
    fn instance_variables() {
        let program = r#"
        obj = Object.new
        obj.instance_variable_set("@foo", "foo")
        obj.instance_variable_set(:@bar, 777)

        def ary_cmp(a,b)
            return false if a - b != []
            return false if b - a != []
            true
        end

        assert(true, ary_cmp([:@foo, :@bar], obj.instance_variables))
        "#;
        let expected = RValue::Nil;
        eval_script(program, expected);
    }

    #[test]
    fn object_send() {
        let program = r#"
        class Foo
            def foo(); "foo" end
            def bar(); "bar" end
            def baz(); "baz" end
        end

        # 任意のキーとメソッド(の名前)の関係をハッシュに保持しておく
        # レシーバの情報がここにはないことに注意
        methods = {1 => :foo, 2 => :bar, 3 => :baz}

        # キーを使って関連するメソッドを呼び出す
        # レシーバは任意(Foo クラスのインスタンスである必要もない)
        assert "foo", Foo.new.send(methods[1])
        assert "bar", Foo.new.send(methods[2])
        assert "baz", Foo.new.send(methods[3])
        "#;
        let expected = RValue::Nil;
        eval_script(program, expected);
    }

    #[test]
    fn object_yield() {
        let program = r#"
        # ブロック付きメソッドの定義、
        # その働きは与えられたブロック(手続き)に引数1, 2を渡して実行すること
        def foo
            yield(1,2)
        end

        # fooに「2引数手続き、その働きは引数を配列に括ってpで印字する」というものを渡して実行させる
        assert [1, 2], foo {|a,b| [a, b]}  # => [1, 2] (要するに p [1, 2] を実行した)
        # 今度は「2引数手続き、その働きは足し算をしてpで印字する」というものを渡して実行させる
        assert 3, foo {|a, b| p a + b}  # => 3 (要するに p 1 + 2 を実行した)

        # 今度のブロック付きメソッドの働きは、
        # 与えられたブロックに引数10を渡して起動し、続けざまに引数20を渡して起動し、
        # さらに引数30を渡して起動すること
        def bar
            a = []
            a << yield(10)
            a << yield(20)
            a << yield(30)
        end

        # barに「1引数手続き、その働きは引数に3を足してpで印字する」というものを渡して実行させる
        assert [13, 23, 33], bar {|v| v + 3 }
        # => 13
        #    23
        #    33 (同じブロックが3つのyieldで3回起動された。
        #        具体的には 10 + 3; 20 + 3; 30 + 3 を実行した)

        "#;
        let expected = RValue::Nil;
        eval_script(program, expected);
    }

    #[test]
    fn object_eval() {
        let program = r#"
        a = 100
        eval("b = 100; assert(100, b);")
        assert(77, eval("a = 77"))
        assert(77, a)
        "#;
        let expected = RValue::Nil;
        eval_script(program, expected);
    }
}
