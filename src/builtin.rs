pub mod array;
pub mod basicobject;
pub mod class;
pub mod comparable;
pub mod complex;
pub mod dir;
pub mod enumerator;
pub mod exception;
pub mod falseclass;
pub mod fiber;
pub mod file;
pub mod float;
pub mod gc;
pub mod hash;
pub mod integer;
pub mod io;
pub mod kernel;
pub mod math;
pub mod method;
pub mod module;
pub mod nilclass;
pub mod numeric;
pub mod object;
pub mod process;
pub mod procobj;
pub mod range;
pub mod regexp;
pub mod string;
pub mod structobj;
pub mod symbol;
pub mod time;
pub mod trueclass;

use crate::*;
use std::cell::RefCell;

thread_local!(
    pub static ESSENTIALS: EssentialClass = EssentialClass::new();
);

thread_local!(
    pub static BUILTINS: RefCell<BuiltinClass> = RefCell::new(BuiltinClass::new());
);

#[derive(Debug, Clone)]
pub struct EssentialClass {
    pub class: Module,
    pub module: Module,
    pub object: Module,
}

impl EssentialClass {
    fn new() -> Self {
        let basic_class = ClassInfo::class_from(None);
        let basic = Module::bootstrap_class(basic_class);
        let object_class = ClassInfo::class_from(basic);
        let object = Module::bootstrap_class(object_class);
        let module_class = ClassInfo::class_from(object);
        let module = Module::bootstrap_class(module_class);
        let class_class = ClassInfo::class_from(module);
        let class = Module::bootstrap_class(class_class);

        basic.set_class(class);
        object.set_class(class);
        module.set_class(class);
        class.set_class(class);

        // Generate singleton class for BasicObject
        let singleton_class = ClassInfo::singleton_from(class, basic);
        let singleton_obj = RValue::new(class, ObjKind::Module(singleton_class)).pack();
        basic.set_class(Module::new(singleton_obj));

        let builtins = EssentialClass {
            class,
            module,
            object,
        };
        builtins
    }
}

#[derive(Debug, Clone)]
pub struct BuiltinClass {
    pub integer: Value,
    pub float: Value,
    pub complex: Value,
    pub array: Value,
    pub symbol: Value,
    pub procobj: Value,
    pub method: Value,
    pub range: Value,
    pub hash: Value,
    pub regexp: Value,
    pub string: Value,
    pub fiber: Value,
    pub enumerator: Value,
    pub exception: Value,
    pub standard: Value,
    pub nilclass: Value,
    pub trueclass: Value,
    pub falseclass: Value,
    pub kernel: Module,
    pub comparable: Module,
    pub numeric: Module,
}

impl BuiltinClass {
    fn new() -> Self {
        // Generate singleton class for BasicObject
        let nil = Value::nil();
        let nilmod = Module::default();
        let builtins = BuiltinClass {
            integer: nil,
            float: nil,
            complex: nil,
            array: nil,
            symbol: nil,
            procobj: nil,
            method: nil,
            range: nil,
            hash: nil,
            regexp: nil,
            string: nil,
            fiber: nil,
            enumerator: nil,
            exception: nil,
            standard: nil,
            nilclass: nil,
            trueclass: nil,
            falseclass: nil,
            kernel: nilmod,
            comparable: nilmod,
            numeric: nilmod,
        };
        builtins
    }

    pub fn initialize() {
        macro_rules! init_builtin {
            ($($module:ident),*) => {$(
                let class_obj = $module::init();
                BUILTINS.with(|m| m.borrow_mut().$module = class_obj);
            )*}
        }
        macro_rules! init {
            ($($module:ident),*) => {$(
                $module::init();
            )*}
        }
        init_builtin!(comparable, numeric, kernel);
        init!(module, class, basicobject, object);
        init_builtin!(float, complex, integer, nilclass, trueclass, falseclass);
        init_builtin!(array, symbol, procobj, range, string, hash);
        init_builtin!(method, regexp, fiber, enumerator);
        init!(math, dir, process, gc, structobj, time);
    }

    /// Bind `object` to the constant `name` of the root object.
    pub(self) fn set_toplevel_constant(name: &str, object: impl Into<Value>) {
        BuiltinClass::object().set_const_by_str(name, object.into());
    }

    /// Get object bound to the constant `name` of the root object.
    pub fn get_toplevel_constant(class_name: &str) -> Option<Value> {
        BuiltinClass::object().get_const_by_str(class_name)
    }

    pub fn object() -> Module {
        ESSENTIALS.with(|m| m.object)
    }

    pub fn class() -> Module {
        ESSENTIALS.with(|m| m.class)
    }

    pub fn module() -> Module {
        ESSENTIALS.with(|m| m.module)
    }

    pub fn string() -> Module {
        BUILTINS.with(|b| b.borrow().string).into_module()
    }

    pub fn integer() -> Module {
        BUILTINS.with(|b| b.borrow().integer).into_module()
    }

    pub fn float() -> Module {
        BUILTINS.with(|b| b.borrow().float).into_module()
    }

    pub fn symbol() -> Module {
        BUILTINS.with(|b| b.borrow().symbol).into_module()
    }

    pub fn complex() -> Module {
        BUILTINS.with(|b| b.borrow().complex).into_module()
    }

    pub fn range() -> Module {
        BUILTINS.with(|b| b.borrow().range).into_module()
    }

    pub fn array() -> Module {
        BUILTINS.with(|b| b.borrow().array).into_module()
    }

    pub fn hash() -> Module {
        BUILTINS.with(|b| b.borrow().hash).into_module()
    }

    pub fn fiber() -> Module {
        BUILTINS.with(|b| b.borrow().fiber).into_module()
    }

    pub fn enumerator() -> Module {
        BUILTINS.with(|b| b.borrow().enumerator).into_module()
    }

    pub fn procobj() -> Module {
        BUILTINS.with(|b| b.borrow().procobj).into_module()
    }

    pub fn regexp() -> Module {
        BUILTINS.with(|b| b.borrow().regexp).into_module()
    }

    pub fn method() -> Module {
        BUILTINS.with(|b| b.borrow().method).into_module()
    }

    pub fn exception() -> Module {
        BUILTINS.with(|b| b.borrow().exception).into_module()
    }

    pub fn standard() -> Module {
        BUILTINS.with(|b| b.borrow().standard).into_module()
    }

    pub fn nilclass() -> Module {
        BUILTINS.with(|b| b.borrow().nilclass).into_module()
    }

    pub fn trueclass() -> Module {
        BUILTINS.with(|b| b.borrow().trueclass).into_module()
    }

    pub fn falseclass() -> Module {
        BUILTINS.with(|b| b.borrow().falseclass).into_module()
    }

    pub fn kernel() -> Module {
        BUILTINS.with(|b| b.borrow().kernel)
    }

    pub fn numeric() -> Module {
        BUILTINS.with(|b| b.borrow().numeric)
    }

    pub fn comparable() -> Module {
        BUILTINS.with(|b| b.borrow().comparable)
    }
}

impl GC for EssentialClass {
    fn mark(&self, alloc: &mut Allocator) {
        self.object.mark(alloc);
    }
}
