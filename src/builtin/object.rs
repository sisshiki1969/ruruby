use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    class: PackedValue,
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
    SplatArray(ArrayRef), // internal use only.
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
            class: PackedValue::nil(), // dummy for boot strapping
            kind: ObjKind::Class(classref),
            var_table: Box::new(HashMap::new()),
        }
    }

    pub fn new_ordinary(class: PackedValue) -> Self {
        ObjectInfo {
            class,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Ordinary,
        }
    }

    pub fn new_class(globals: &Globals, classref: ClassRef) -> Self {
        ObjectInfo {
            class: globals.class,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Class(classref),
        }
    }

    pub fn new_module(globals: &Globals, classref: ClassRef) -> Self {
        ObjectInfo {
            class: globals.module,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Module(classref),
        }
    }

    pub fn new_array(globals: &Globals, arrayref: ArrayRef) -> Self {
        ObjectInfo {
            class: globals.array,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Array(arrayref),
        }
    }

    pub fn new_splat(globals: &Globals, arrayref: ArrayRef) -> Self {
        ObjectInfo {
            class: globals.array,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::SplatArray(arrayref),
        }
    }

    pub fn new_hash(globals: &Globals, hashref: HashRef) -> Self {
        ObjectInfo {
            class: globals.hash,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Hash(hashref),
        }
    }

    pub fn new_regexp(globals: &Globals, regexpref: RegexpRef) -> Self {
        ObjectInfo {
            class: globals.regexp,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Regexp(regexpref),
        }
    }

    pub fn new_range(globals: &Globals, info: RangeInfo) -> Self {
        ObjectInfo {
            class: globals.range,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Range(info),
        }
    }

    pub fn new_proc(globals: &Globals, procref: ProcRef) -> Self {
        ObjectInfo {
            class: globals.procobj,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Proc(procref),
        }
    }

    pub fn new_method(globals: &Globals, methodref: MethodObjRef) -> Self {
        ObjectInfo {
            class: globals.method,
            var_table: Box::new(HashMap::new()),
            kind: ObjKind::Method(methodref),
        }
    }
}

pub type ObjectRef = Ref<ObjectInfo>;

impl ObjectRef {
    pub fn class(&self) -> PackedValue {
        self.class
    }

    pub fn search_class(&self) -> PackedValue {
        let mut class = self.class;
        loop {
            if class.as_class().is_singleton {
                class = class.as_object().class;
            } else {
                return class;
            }
        }
    }

    pub fn set_class(&mut self, class: PackedValue) {
        self.class = class;
    }

    pub fn get_var(&self, id: IdentId) -> Option<PackedValue> {
        self.var_table.get(&id).cloned()
    }

    pub fn get_mut_var(&mut self, id: IdentId) -> Option<&mut PackedValue> {
        self.var_table.get_mut(&id)
    }

    pub fn set_var(&mut self, id: IdentId, val: PackedValue) {
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

    {
        use std::env;
        let id = globals.get_ident_id("ARGV");
        let res = env::args()
            .enumerate()
            .filter(|(i, _)| *i > 1)
            .map(|(_, x)| PackedValue::string(x))
            .collect();
        let argv = PackedValue::array_from(&globals, res);
        globals.object.set_var(id, argv);
    }
}

fn class(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let class = args.self_value.get_class_object(&vm.globals);
    Ok(class)
}

fn object_id(_vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let id = args.self_value.id();
    Ok(PackedValue::fixnum(id as i64))
}

fn singleton_class(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.get_singleton_class(args.self_value)
}

fn inspect(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let inspect = vm.val_pp(args.self_value);
    Ok(PackedValue::string(inspect))
}

fn eql(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    Ok(PackedValue::bool(args.self_value == args[0]))
}

fn toi(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
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
    Ok(PackedValue::fixnum(num))
}

fn instance_variable_set(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
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

fn instance_variables(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut receiver = args.self_value.as_object();
    let res = receiver
        .var_table()
        .keys()
        .filter(|x| vm.globals.get_ident_name(**x).chars().nth(0) == Some('@'))
        .map(|x| PackedValue::symbol(*x))
        .collect();
    Ok(PackedValue::array_from(&vm.globals, res))
}

fn floor(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let rec = args.self_value;
    if rec.is_packed_fixnum() {
        Ok(rec)
    } else if rec.is_packed_num() {
        let res = rec.as_packed_flonum().floor() as i64;
        Ok(PackedValue::fixnum(res))
    } else {
        Err(vm.error_type("Receiver must be Integer of Float."))
    }
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
        let expected = Value::Nil;
        eval_script(program, expected);
    }
}
