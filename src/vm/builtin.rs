use super::value::*;
use crate::vm::*;

pub struct Builtin {}

impl Builtin {
    pub fn init_builtin(globals: &mut Globals) {
        globals.add_builtin_method("chr", builtin_chr);
        globals.add_builtin_method("puts", builtin_puts);
        globals.add_builtin_method("print", builtin_print);
        globals.add_builtin_method("assert", builtin_assert);

        /// Built-in function "chr".
        fn builtin_chr(_vm: &mut VM, receiver: PackedValue, _args: Vec<PackedValue>) -> VMResult {
            if receiver.is_packed_fixnum() {
                let i = receiver.as_packed_fixnum();
                Ok(Value::Char(i as u8).pack())
            } else {
                match receiver.unpack() {
                    Value::FixNum(i) => Ok(Value::Char(i as u8).pack()),
                    _ => unimplemented!(),
                }
            }
        }

        /// Built-in function "puts".
        fn builtin_puts(vm: &mut VM, _receiver: PackedValue, args: Vec<PackedValue>) -> VMResult {
            fn flatten(vm: &VM, val: PackedValue) {
                match val.as_array() {
                    None => println!("{}", vm.val_to_s(val)),
                    Some(aref) => {
                        for val in &aref.elements {
                            flatten(vm, val.clone());
                        }
                    }
                }
            }
            for arg in args {
                flatten(vm, arg);
            }
            Ok(PackedValue::nil())
        }

        /// Built-in function "print".
        fn builtin_print(vm: &mut VM, _receiver: PackedValue, args: Vec<PackedValue>) -> VMResult {
            for arg in args {
                if let Value::Char(ch) = arg.unpack() {
                    let v = [ch];
                    use std::io::{self, Write};
                    io::stdout().write(&v).unwrap();
                } else {
                    print!("{}", vm.val_to_s(arg));
                }
            }
            Ok(PackedValue::nil())
        }

        /// Built-in function "assert".
        fn builtin_assert(vm: &mut VM, _receiver: PackedValue, args: Vec<PackedValue>) -> VMResult {
            if args.len() != 2 {
                panic!("Invalid number of arguments.");
            }
            if !vm.eval_eq(args[0].clone(), args[1].clone())? {
                panic!(
                    "Assertion error: Expected: {:?} Actual: {:?}",
                    args[0].unpack(),
                    args[1].unpack()
                );
            } else {
                println!("Assert OK: {:?}", vm.val_pp(args[0]));
                Ok(PackedValue::nil())
            }
        }
    }
}
