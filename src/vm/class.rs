use crate::util::*;
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: Option<IdentId>,
    pub method_table: MethodTable,
    pub superclass: Value,
    pub include: Vec<Value>,
    pub is_singleton: bool,
}

impl ClassInfo {
    pub fn new(name: impl Into<Option<IdentId>>, superclass: Value) -> Self {
        ClassInfo {
            name: name.into(),
            method_table: HashMap::new(),
            superclass,
            include: vec![],
            is_singleton: false,
        }
    }
}

pub type ClassRef = Ref<ClassInfo>;

impl ClassRef {
    pub fn from(id: impl Into<Option<IdentId>>, superclass: impl Into<Option<Value>>) -> Self {
        let superclass = match superclass.into() {
            Some(superclass) => superclass,
            None => Value::nil(),
        };
        ClassRef::new(ClassInfo::new(id, superclass))
    }

    pub fn superclass(&self) -> Option<ClassRef> {
        if self.superclass.is_nil() {
            None
        } else {
            Some(self.superclass.as_class())
        }
    }
}

pub fn init_class(globals: &mut Globals) {
    let class = globals.class_class;
    globals.add_builtin_instance_method(class, "new", new);
    globals.add_builtin_instance_method(class, "superclass", superclass);
    globals.add_builtin_class_method(globals.builtins.class, "new", class_new);
}

// Class methods

/// Create new class.
/// If a block is given, eval it in the context of newly created class.
/// args[0]: super class.
fn class_new(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 1)?;
    let superclass = if args.len() == 0 {
        vm.globals.builtins.object
    } else {
        args[0]
    };
    let id = vm.globals.get_ident_id("nil");
    let classref = ClassRef::from(id, superclass);
    let val = Value::class(&mut vm.globals, classref);

    match args.block {
        Some(method) => {
            vm.class_push(val);
            let arg = Args::new1(val, None, val);
            vm.eval_block(method, &arg)?;
            vm.class_pop();
        }
        None => {}
    };
    Ok(val)
}

/// Create new instance of `self`.
fn new(vm: &mut VM, args: &Args) -> VMResult {
    let new_instance = Value::ordinary_object(args.self_value);
    // Call initialize method if it exists.
    if let Some(method) = args.self_value.get_instance_method(IdentId::INITIALIZE) {
        let mut args = args.clone();
        args.self_value = new_instance;
        vm.eval_send(method, &args)?;
    };
    Ok(new_instance)
}

/// Get super class of `self`.
fn superclass(vm: &mut VM, args: &Args) -> VMResult {
    let class = vm.val_as_class(args.self_value, "Receiver")?;
    Ok(class.superclass)
}
