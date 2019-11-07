use super::value::Value;
use crate::vm::VMResult;
use crate::vm::*;

pub struct Builtin {}

impl Builtin {
    pub fn init_builtin(globals: &mut Globals) {
        globals.add_builtin_method("chr", builtin_chr);
        globals.add_builtin_method("puts", builtin_puts);
        globals.add_builtin_method("print", builtin_print);
        globals.add_builtin_method("assert", builtin_assert);

        /// Built-in function "chr".
        pub fn builtin_chr(_vm: &mut VM, receiver: Value, _args: Vec<Value>) -> VMResult {
            match receiver {
                Value::FixNum(i) => Ok(Value::Char(i as u8)),
                _ => unimplemented!(),
            }
        }

        /// Built-in function "puts".
        pub fn builtin_puts(vm: &mut VM, _receiver: Value, args: Vec<Value>) -> VMResult {
            for arg in args {
                println!("{}", vm.val_to_s(&arg));
            }
            Ok(Value::Nil)
        }

        /// Built-in function "print".
        pub fn builtin_print(vm: &mut VM, _receiver: Value, args: Vec<Value>) -> VMResult {
            for arg in args {
                if let Value::Char(ch) = arg {
                    let v = [ch];
                    use std::io::{self, Write};
                    io::stdout().write(&v).unwrap();
                } else {
                    print!("{}", vm.val_to_s(&arg));
                }
            }
            Ok(Value::Nil)
        }

        /// Built-in function "assert".
        pub fn builtin_assert(vm: &mut VM, _receiver: Value, args: Vec<Value>) -> VMResult {
            if args.len() != 2 {
                panic!("Invalid number of arguments.");
            }
            if vm.eval_eq(args[0].clone(), args[1].clone())? != Value::Bool(true) {
                panic!(
                    "Assertion error: Expected: {:?} Actual: {:?}",
                    args[0], args[1]
                );
            } else {
                Ok(Value::Nil)
            }
        }
    }
    /// Built-in function "new".
    pub fn builtin_new(vm: &mut VM, receiver: Value, _args: Vec<Value>) -> VMResult {
        match receiver {
            Value::Class(class_ref) => {
                let instance = vm.globals.new_instance(class_ref);
                Ok(Value::Instance(instance))
            }
            _ => Err(vm.error_unimplemented(format!("Receiver must be a class! {:?}", receiver))),
        }
    }
}
