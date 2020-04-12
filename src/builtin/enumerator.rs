use crate::*;

#[derive(Debug, Clone)]
pub struct EnumInfo {
    base: Value,
    method: IdentId,
    args: Args,
}

impl EnumInfo {
    pub fn new(base: Value, method: IdentId, args: Args) -> Self {
        EnumInfo { base, method, args }
    }
}

pub type EnumRef = Ref<EnumInfo>;

impl EnumRef {
    pub fn from(base: Value, method: IdentId, args: Args) -> Self {
        EnumRef::new(EnumInfo::new(base, method, args))
    }
}

pub fn init_enumerator(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Enumerator");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "each", each);
    let class = Value::class(globals, class);
    globals.add_builtin_class_method(class, "new", enum_new);
    class
}

// Class methods

fn enum_new(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 65535)?;
    let obj = args[0];
    let (method, new_args) = if args.len() == 1 {
        let method = vm.globals.get_ident_id("each");
        let new_args = Args::new0(args.self_value, None);
        (method, new_args)
    } else {
        if !args[1].is_packed_symbol() {
            return Err(vm.error_argument("2nd arg must be Symbol."));
        };
        let method = args[1].as_packed_symbol();
        let mut new_args = Args::new(args.len() - 2);
        for i in 0..args.len() - 2 {
            new_args[i] = args[i + 2];
        }
        new_args.self_value = args.self_value;
        new_args.block = None;
        (method, new_args)
    };
    let val = Value::enumerator(&vm.globals, obj, method, new_args);
    Ok(val)
}

// Instance methods

fn each(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    Ok(Value::nil())
}
