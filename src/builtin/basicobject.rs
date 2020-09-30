use crate::*;

pub fn init(globals: &mut Globals) {
    let _basic_object = globals.builtins.object.superclass().unwrap();
    // class.add_builtin_method_by_str("count", count);

    // class_obj.add_builtin_class_method("new", array_new);
}
