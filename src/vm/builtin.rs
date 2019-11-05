use super::codegen::{MethodInfo, MethodTable};
use super::value::Value;
use crate::util::IdentifierTable;
use crate::vm::VMResult;
use crate::vm::VM;

pub struct Builtin {}

impl Builtin {
    pub fn init_builtin(ident_table: &mut IdentifierTable, method_table: &mut MethodTable) {
        let id = ident_table.get_ident_id(&"chr".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "chr".to_string(),
            func: builtin_chr,
        };
        method_table.insert(id, info);

        let id = ident_table.get_ident_id(&"puts".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "puts".to_string(),
            func: builtin_puts,
        };
        method_table.insert(id, info);

        let id = ident_table.get_ident_id(&"print".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "print".to_string(),
            func: builtin_print,
        };
        method_table.insert(id, info);

        let id = ident_table.get_ident_id(&"assert".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "assert".to_string(),
            func: builtin_assert,
        };
        method_table.insert(id, info);

        /// Built-in function "chr".
        pub fn builtin_chr(_eval: &mut VM, receiver: Value, _args: Vec<Value>) -> VMResult {
            match receiver {
                Value::FixNum(i) => Ok(Value::Char(i as u8)),
                _ => unimplemented!(),
            }
        }

        /// Built-in function "puts".
        pub fn builtin_puts(eval: &mut VM, _receiver: Value, args: Vec<Value>) -> VMResult {
            for arg in args {
                println!("{}", eval.val_to_s(&arg));
            }
            Ok(Value::Nil)
        }

        /// Built-in function "print".
        pub fn builtin_print(eval: &mut VM, _receiver: Value, args: Vec<Value>) -> VMResult {
            for arg in args {
                if let Value::Char(ch) = arg {
                    let v = [ch];
                    use std::io::{self, Write};
                    io::stdout().write(&v).unwrap();
                } else {
                    print!("{}", eval.val_to_s(&arg));
                }
            }
            Ok(Value::Nil)
        }

        /// Built-in function "assert".
        pub fn builtin_assert(eval: &mut VM, _receiver: Value, args: Vec<Value>) -> VMResult {
            if args.len() != 2 {
                panic!("Invalid number of arguments.");
            }
            if eval.eval_eq(args[0].clone(), args[1].clone())? != Value::Bool(true) {
                panic!(
                    "Assertion error: Expected: {:?} Actual: {:?}",
                    args[0], args[1]
                );
            } else {
                Ok(Value::Nil)
            }
        }

        /*
        /// Built-in function "new".
        pub fn builtin_new(eval: &mut VM, receiver: Value, _args: Vec<Value>) -> VMResult {
            match receiver {
                Value::Class(class_ref) => {
                    let instance = eval.new_instance(class_ref);
                    Ok(Value::Instance(instance))
                }
                _ => Err(eval.error_unimplemented(
                    format!("Receiver must be a class! {:?}", receiver),
                    eval.loc,
                )),
            }
        }
        */
    }
}
