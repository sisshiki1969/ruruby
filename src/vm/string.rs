use crate::vm::*;

pub fn init_string(globals: &mut Globals) -> ClassRef {
    let id = globals.get_ident_id("String");
    let class = ClassRef::from(id, globals.object_class);
    /*
    globals.add_builtin_instance_method(class, "map", range_map);
    globals.add_builtin_instance_method(class, "begin", range_begin);
    globals.add_builtin_instance_method(class, "first", range_first);
    globals.add_builtin_instance_method(class, "end", range_end);
    globals.add_builtin_instance_method(class, "last", range_last);
    globals.add_builtin_class_method(class, "new", range_new);
    */
    class
}
