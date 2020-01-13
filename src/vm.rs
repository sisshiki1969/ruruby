mod array;
mod builtin;
mod class;
mod codegen;
mod context;
mod globals;
mod hash;
mod integer;
mod method;
mod module;
mod object;
#[cfg(feature = "perf")]
mod perf;
mod procobj;
mod range;
mod string;
pub mod value;
mod vm_inst;

use crate::error::{RubyError, RuntimeErrKind};
use crate::parser::*;
pub use crate::parser::{LvarCollector, LvarId, ParseResult};
pub use crate::util::*;
use array::*;
pub use class::*;
use codegen::{Codegen, ISeq, ISeqPos};
pub use context::*;
pub use globals::*;
pub use hash::*;
pub use method::*;
pub use module::*;
use object::*;
#[cfg(feature = "perf")]
use perf::*;
use range::*;
use std::collections::HashMap;
use std::path::PathBuf;
pub use value::*;
use vm_inst::*;

pub type ValueTable = HashMap<IdentId, PackedValue>;

pub type VMResult = Result<PackedValue, RubyError>;

#[derive(Debug, Clone)]
pub struct VM {
    // Global info
    pub globals: Globals,
    pub root_path: Vec<PathBuf>,
    // VM state
    pub context_stack: Vec<ContextRef>,
    pub class_stack: Vec<ClassRef>,
    pub exec_stack: Vec<PackedValue>,
    pub pc: usize,
    #[cfg(feature = "perf")]
    perf: Perf,
}

impl VM {
    pub fn new() -> Self {
        let mut globals = Globals::new();

        macro_rules! set_builtin_class {
            ($name:expr, $class:ident) => {
                let id = globals.get_ident_id($name);
                let class = PackedValue::class(&globals, globals.$class);
                globals.object_class.constants.insert(id, class);
            };
        }

        set_builtin_class!("Object", object_class);
        set_builtin_class!("Module", module_class);
        set_builtin_class!("Class", class_class);
        set_builtin_class!("Integer", integer_class);
        set_builtin_class!("Array", array_class);
        set_builtin_class!("Proc", proc_class);
        set_builtin_class!("Range", range_class);
        set_builtin_class!("String", string_class);
        set_builtin_class!("Hash", hash_class);
        set_builtin_class!("Method", method_class);

        let id = globals.get_ident_id("StandardError");
        let class = PackedValue::class(&globals, globals.class_class);
        globals.object_class.constants.insert(id, class);

        let mut vm = VM {
            globals,
            root_path: vec![],
            class_stack: vec![],
            context_stack: vec![],
            exec_stack: vec![],
            pc: 0,
            #[cfg(feature = "perf")]
            perf: Perf::new(),
        };
        builtin::Builtin::init_builtin(&mut vm.globals);
        vm
    }

    pub fn context(&self) -> ContextRef {
        *self.context_stack.last().unwrap()
    }

    pub fn source_info(&self) -> SourceInfoRef {
        self.context().iseq_ref.source_info
    }

    pub fn class(&self) -> ClassRef {
        if self.class_stack.len() == 0 {
            self.globals.object_class
        } else {
            *self.class_stack.last().unwrap()
        }
    }

    /// Get local variable table.
    pub fn get_outer_context(&mut self, outer: u32) -> ContextRef {
        let mut context = self.context();
        for _ in 0..outer {
            context = context.outer.unwrap();
        }
        context
    }

    pub fn run(&mut self, path: impl Into<String>, program: String) -> VMResult {
        let mut parser = Parser::new();
        std::mem::swap(&mut parser.ident_table, &mut self.globals.ident_table);
        let result = parser.parse_program(path, program)?;
        self.globals.ident_table = result.ident_table;

        #[cfg(feature = "perf")]
        {
            self.perf.set_prev_inst(Perf::CODEGEN);
        }
        let methodref = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            &vec![],
            &result.node,
            &result.lvar_collector,
            true,
            false,
        )?;
        let iseq = self.globals.get_method_info(methodref).as_iseq(&self)?;
        let main = self.globals.main_object;
        let main_object = PackedValue::object(main);
        self.vm_run(main_object, iseq, None, VecArray::new0(), None, None)?;
        let val = self.exec_stack.pop().unwrap();
        #[cfg(feature = "perf")]
        {
            self.perf.get_perf(Perf::INVALID);
        }
        let stack_len = self.exec_stack.len();
        if stack_len != 0 {
            eprintln!("Error: stack length is illegal. {}", stack_len);
        };
        #[cfg(feature = "perf")]
        {
            self.perf.print_perf();
        }
        Ok(val)
    }

    pub fn run_repl(&mut self, result: &ParseResult, mut context: ContextRef) -> VMResult {
        #[cfg(feature = "perf")]
        {
            self.perf.set_prev_inst(Perf::CODEGEN);
        }
        self.globals.ident_table = result.ident_table.clone();
        let methodref = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            &vec![],
            &result.node,
            &result.lvar_collector,
            true,
            false,
        )?;
        let iseq = self.globals.get_method_info(methodref).as_iseq(&self)?;
        context.iseq_ref = iseq;
        context.adjust_lvar_size();

        self.vm_run_context(context)?;
        let val = self.exec_stack.pop().unwrap();
        #[cfg(feature = "perf")]
        {
            self.perf.get_perf(Perf::INVALID);
        }
        let stack_len = self.exec_stack.len();
        if stack_len != 0 {
            eprintln!("Error: stack length is illegal. {}", stack_len);
        };
        #[cfg(feature = "perf")]
        {
            self.perf.print_perf();
        }
        Ok(val)
    }

    /// Create new context from given args, and run vm on the context.
    pub fn vm_run(
        &mut self,
        self_value: PackedValue,
        iseq: ISeqRef,
        outer: Option<ContextRef>,
        args: VecArray,
        kw_arg: Option<PackedValue>,
        block: Option<MethodRef>,
    ) -> Result<(), RubyError> {
        self.check_args_num(args.len(), iseq.min_params, iseq.max_params)?;
        let mut context = Context::new(self_value, block, iseq, outer);
        context.set_arguments(&self.globals, args);
        if let Some(id) = iseq.lvar.block_param() {
            *context.get_mut_lvar(id) = match block {
                Some(block) => {
                    let proc_context = self.create_context_from_method(block)?;
                    PackedValue::procobj(&self.globals, proc_context)
                }
                None => PackedValue::nil(),
            }
        }
        match kw_arg {
            Some(kw_arg) => {
                let keyword = kw_arg.as_hash().unwrap();
                for (k, v) in keyword.map.iter() {
                    eprintln!("{} {}", self.val_pp(*k), self.val_pp(*v));
                    let id = k.as_symbol().unwrap();
                    match iseq.keyword_params.get(&id) {
                        Some(lvar) => {
                            *context.get_mut_lvar(*lvar) = *v;
                        }
                        None => return Err(self.error_argument("Undefined keyword.")),
                    };
                }
            }
            None => {}
        };
        self.vm_run_context(ContextRef::new_local(&context))
    }

    pub fn vm_run_context(&mut self, context: ContextRef) -> Result<(), RubyError> {
        self.context_stack.push(context);
        let old_pc = self.pc;
        self.pc = context.pc;
        let iseq = &context.iseq_ref.iseq;
        loop {
            #[cfg(feature = "perf")]
            {
                self.perf.get_perf(iseq[self.pc]);
            }
            #[cfg(feature = "trace")]
            {
                println!(
                    "{} stack:{}",
                    Inst::inst_name(iseq[self.pc]),
                    self.exec_stack.len()
                );
            }
            match iseq[self.pc] {
                Inst::END | Inst::RETURN => {
                    self.context_stack.pop().unwrap();
                    self.pc = old_pc;
                    return Ok(());
                }
                Inst::PUSH_NIL => {
                    self.exec_stack.push(PackedValue::nil());
                    self.pc += 1;
                }
                Inst::PUSH_TRUE => {
                    self.exec_stack.push(PackedValue::true_val());
                    self.pc += 1;
                }
                Inst::PUSH_FALSE => {
                    self.exec_stack.push(PackedValue::false_val());
                    self.pc += 1;
                }
                Inst::PUSH_SELF => {
                    self.exec_stack.push(context.self_value);
                    self.pc += 1;
                }
                Inst::PUSH_FIXNUM => {
                    let num = read64(iseq, self.pc + 1);
                    self.pc += 9;
                    self.exec_stack.push(PackedValue::fixnum(num as i64));
                }
                Inst::PUSH_FLONUM => {
                    let num = unsafe { std::mem::transmute(read64(iseq, self.pc + 1)) };
                    self.pc += 9;
                    self.exec_stack.push(PackedValue::flonum(num));
                }
                Inst::PUSH_STRING => {
                    let id = read_id(iseq, self.pc + 1);
                    let string = self.globals.get_ident_name(id).clone();
                    self.exec_stack.push(PackedValue::string(string));
                    self.pc += 5;
                }
                Inst::PUSH_SYMBOL => {
                    let id = read_id(iseq, self.pc + 1);
                    self.exec_stack.push(PackedValue::symbol(id));
                    self.pc += 5;
                }

                Inst::ADD => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    self.eval_add(lhs, rhs)?;
                    self.pc += 1;
                }
                Inst::ADDI => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let i = read32(iseq, self.pc + 1) as i32;
                    self.eval_addi(lhs, i)?;
                    self.pc += 5;
                }
                Inst::SUB => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    self.eval_sub(lhs, rhs)?;
                    self.pc += 1;
                }
                Inst::SUBI => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let i = read32(iseq, self.pc + 1) as i32;
                    self.eval_subi(lhs, i)?;
                    self.pc += 5;
                }
                Inst::MUL => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    self.eval_mul(lhs, rhs)?;
                    self.pc += 1;
                }
                Inst::DIV => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_div(lhs, rhs)?;
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::REM => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_rem(lhs, rhs)?;
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::SHR => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_shr(lhs, rhs)?;
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::SHL => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_shl(lhs, rhs)?;
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::BIT_AND => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_bitand(lhs, rhs)?;
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::BIT_OR => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_bitor(lhs, rhs)?;
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::BIT_XOR => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_bitxor(lhs, rhs)?;
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::BIT_NOT => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_bitnot(lhs)?;
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::EQ => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = PackedValue::bool(self.eval_eq(lhs, rhs)?);
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::NE => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = PackedValue::bool(!self.eval_eq(lhs, rhs)?);
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::TEQ => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let res = match lhs.as_class() {
                        Some(class) if rhs.get_class(&self.globals) == class => true,
                        _ => match self.eval_eq(lhs, rhs) {
                            Ok(res) => res,
                            Err(_) => false,
                        },
                    };
                    let val = PackedValue::bool(res);
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::GT => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_gt(lhs, rhs)?;
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::GE => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_ge(lhs, rhs)?;
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::NOT => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let val = PackedValue::bool(!self.val_to_bool(lhs));
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::CONCAT_STRING => {
                    let rhs = self.exec_stack.pop().unwrap().as_string();
                    let lhs = self.exec_stack.pop().unwrap().as_string();
                    let val = match (lhs, rhs) {
                        (Some(lhs), Some(rhs)) => PackedValue::string(format!("{}{}", lhs, rhs)),
                        (_, _) => unreachable!("Illegal CAONCAT_STRING arguments."),
                    };
                    self.exec_stack.push(val);
                    self.pc += 1;
                }
                Inst::SET_LOCAL => {
                    let id = read_lvar_id(iseq, self.pc + 1);
                    let outer = read32(iseq, self.pc + 5);
                    let val = self.exec_stack.pop().unwrap();
                    let mut cref = self.get_outer_context(outer);
                    *cref.get_mut_lvar(id) = val;
                    self.pc += 9;
                }
                Inst::GET_LOCAL => {
                    let id = read_lvar_id(iseq, self.pc + 1);
                    let outer = read32(iseq, self.pc + 5);
                    let cref = self.get_outer_context(outer);
                    let val = cref.get_lvar(id);
                    self.exec_stack.push(val);
                    self.pc += 9;
                }
                Inst::CHECK_LOCAL => {
                    let id = read_lvar_id(iseq, self.pc + 1);
                    let outer = read32(iseq, self.pc + 5);
                    let cref = self.get_outer_context(outer);
                    let val = cref.get_lvar(id).is_uninitialized();
                    self.exec_stack.push(PackedValue::bool(val));
                    self.pc += 9;
                }
                Inst::SET_CONST => {
                    let id = read_id(iseq, self.pc + 1);
                    let val = self.exec_stack.pop().unwrap();
                    let mut class = self.class();
                    class.constants.insert(id, val);
                    self.pc += 5;
                }
                Inst::GET_CONST => {
                    let id = read32(iseq, self.pc + 1);
                    let class = self.class();
                    let val = if id == 0 {
                        PackedValue::class(&self.globals, class)
                    } else {
                        self.get_constant(class, IdentId::from(id))?
                    };
                    self.exec_stack.push(val);
                    self.pc += 5;
                }
                Inst::GET_CONST_TOP => {
                    let id = read_id(iseq, self.pc + 1);
                    let class = self.globals.object_class;
                    let val = self.get_constant(class, id)?;
                    self.exec_stack.push(val);
                    self.pc += 5;
                }
                Inst::GET_SCOPE => {
                    let parent = self.exec_stack.pop().unwrap();
                    let id = read_id(iseq, self.pc + 1);
                    let class = match parent.as_module() {
                        Some(class) => class,
                        None => {
                            return Err(self.error_type(format!(
                                "{:?} is not a class/module.",
                                parent.unpack()
                            )))
                        }
                    };
                    let val = self.get_constant(class, id)?;
                    self.exec_stack.push(val);
                    self.pc += 5;
                }
                Inst::SET_INSTANCE_VAR => {
                    let var_id = read_id(iseq, self.pc + 1);
                    let mut self_obj = self.context().self_value.as_object().unwrap();
                    let new_val = self.exec_stack.pop().unwrap();
                    self_obj.instance_var.insert(var_id, new_val);
                    self.pc += 5;
                }
                Inst::GET_INSTANCE_VAR => {
                    let var_id = read_id(iseq, self.pc + 1);
                    let self_obj = self.context().self_value.as_object().unwrap();
                    let val = match self_obj.instance_var.get(&var_id) {
                        Some(val) => val.clone(),
                        None => PackedValue::nil(),
                    };
                    self.exec_stack.push(val);
                    self.pc += 5;
                }
                Inst::SET_ARRAY_ELEM => {
                    let arg_num = read32(iseq, self.pc + 1) as usize;
                    let args = self.pop_args(arg_num);
                    let arg_num = args.len();
                    match self.exec_stack.pop().unwrap().as_object() {
                        Some(oref) => {
                            match oref.kind {
                                ObjKind::Array(mut aref) => {
                                    self.check_args_num(arg_num, 1, 2)?;
                                    let index = args[0].expect_fixnum(&self, "Index")?;
                                    if arg_num == 1 && index >= aref.elements.len() as i64 {
                                        let padding = index as usize - aref.elements.len();
                                        aref.elements
                                            .append(&mut vec![PackedValue::nil(); padding]);
                                        aref.elements.push(self.exec_stack.pop().unwrap());
                                    } else {
                                        let index =
                                            self.get_array_index(index, aref.elements.len())?;
                                        let val = self.exec_stack.pop().unwrap();
                                        if arg_num == 1 {
                                            aref.elements[index] = val;
                                        } else {
                                            let len = args[1].expect_fixnum(&self, "Index")?;
                                            if len < 0 {
                                                return Err(self.error_index(format!(
                                                    "Negative length. {}",
                                                    len
                                                )));
                                            } else {
                                                let len = len as usize;
                                                let end =
                                                    std::cmp::min(aref.elements.len(), index + len);
                                                aref.elements.drain(index..end);
                                                aref.elements.insert(index, val);
                                            }
                                        }
                                    }
                                }
                                ObjKind::Hash(mut href) => {
                                    let key = args[0];
                                    let val = self.exec_stack.pop().unwrap();
                                    href.map.insert(key, val);
                                }
                                _ => {
                                    return Err(self.error_unimplemented(
                                        "Currently, []= is supported only for Array and Hash.",
                                    ))
                                }
                            };
                        }
                        None => {
                            return Err(self.error_unimplemented(
                                "Currently, []= is supported only for Array and Hash.",
                            ))
                        }
                    }

                    self.pc += 5;
                }
                Inst::GET_ARRAY_ELEM => {
                    let arg_num = read32(iseq, self.pc + 1) as usize;
                    let args = self.pop_args(arg_num);
                    let arg_num = args.len();
                    match self.exec_stack.pop().unwrap().unpack() {
                        Value::Object(oref) => {
                            match oref.kind {
                                ObjKind::Array(aref) => {
                                    self.check_args_num(arg_num, 1, 2)?;
                                    let index = args[0].expect_fixnum(&self, "Index")?;
                                    let index = self.get_array_index(index, aref.elements.len())?;
                                    if arg_num == 1 {
                                        let elem = aref.elements[index];
                                        self.exec_stack.push(elem);
                                    } else {
                                        let len = args[1].expect_fixnum(&self, "Index")?;
                                        if len < 0 {
                                            self.exec_stack.push(PackedValue::nil());
                                        } else {
                                            let len = len as usize;
                                            let end =
                                                std::cmp::min(aref.elements.len(), index + len);
                                            let ary = (&aref.elements[index..end]).to_vec();
                                            let ary_object = PackedValue::array(
                                                &self.globals,
                                                ArrayRef::from(ary),
                                            );
                                            self.exec_stack.push(ary_object);
                                        }
                                    };
                                }
                                ObjKind::Hash(href) => {
                                    self.check_args_num(arg_num, 1, 2)?;
                                    let key = args[0];
                                    let val = match href.map.get(&key) {
                                        Some(val) => val.clone(),
                                        None => PackedValue::nil(),
                                    };
                                    self.exec_stack.push(val);
                                }
                                _ => {
                                    return Err(self.error_unimplemented(
                                        "Currently, [] is supported only for Array and Hash.",
                                    ))
                                }
                            };
                        }
                        Value::FixNum(i) => {
                            self.check_args_num(arg_num, 1, 1)?;
                            let index = args[0].expect_fixnum(&self, "Index")?;
                            let val = if index < 0 || 63 < index {
                                0
                            } else {
                                (i >> index) & 1
                            };
                            self.exec_stack.push(PackedValue::fixnum(val));
                        }
                        _ => {
                            return Err(self.error_unimplemented(
                                "Currently, [] is supported only for Array and Hash.",
                            ))
                        }
                    }
                    self.pc += 5;
                }
                Inst::SPLAT => {
                    let array = self.exec_stack.pop().unwrap();
                    let res = match array.as_array() {
                        Some(array) => PackedValue::splat(&self.globals, array),
                        None => array,
                    };
                    self.exec_stack.push(res);
                    self.pc += 1;
                }
                Inst::CREATE_RANGE => {
                    let start = self.exec_stack.pop().unwrap();
                    let end = self.exec_stack.pop().unwrap();
                    match (start.unpack(), end.unpack()) {
                        (Value::FixNum(_), Value::FixNum(_)) => {}
                        _ => return Err(self.error_argument("Bad value for range.")),
                    };
                    let exclude_val = self.exec_stack.pop().unwrap();
                    let exclude_end = self.val_to_bool(exclude_val);
                    let range = PackedValue::range(&mut self.globals, start, end, exclude_end);
                    self.exec_stack.push(range);
                    self.pc += 1;
                }
                Inst::CREATE_ARRAY => {
                    let arg_num = read32(iseq, self.pc + 1) as usize;
                    let elems = self.pop_args(arg_num);
                    let array = PackedValue::array(&mut self.globals, ArrayRef::from(elems));
                    self.exec_stack.push(array);
                    self.pc += 5;
                }
                Inst::CREATE_PROC => {
                    let method = MethodRef::from(read32(iseq, self.pc + 1));
                    let proc_obj = self.create_proc_obj(method)?;
                    self.exec_stack.push(proc_obj);
                    self.pc += 5;
                }
                Inst::CREATE_HASH => {
                    let arg_num = read32(iseq, self.pc + 1) as usize;
                    let key_value = self.pop_key_value_pair(arg_num);
                    let hash = PackedValue::hash(&self.globals, HashRef::from(key_value));
                    self.exec_stack.push(hash);
                    self.pc += 5;
                }
                Inst::JMP => {
                    let disp = read32(iseq, self.pc + 1) as i32 as i64;
                    self.pc = ((self.pc as i64) + 5 + disp) as usize;
                }
                Inst::JMP_IF_FALSE => {
                    let val = self.exec_stack.pop().unwrap();
                    if self.val_to_bool(val) {
                        self.pc += 5;
                    } else {
                        let disp = read32(iseq, self.pc + 1) as i32 as i64;
                        self.pc = ((self.pc as i64) + 5 + disp) as usize;
                    }
                }
                Inst::SEND => {
                    let receiver = self.exec_stack.pop().unwrap();
                    let method_id = read_id(iseq, self.pc + 1);
                    let args_num = read32(iseq, self.pc + 5) as usize;
                    let kw_args_num = read32(iseq, self.pc + 9) as usize;
                    let cache_slot = read32(iseq, self.pc + 13) as usize;
                    let block = read32(iseq, self.pc + 17);
                    let methodref = self.get_method_from_cache(cache_slot, receiver, method_id)?;

                    let keyword = if kw_args_num != 0 {
                        let val = self.exec_stack.pop().unwrap();
                        eprintln!("{}", self.val_pp(val));
                        Some(val)
                    } else {
                        None
                    };
                    let args = self.pop_args_to_ary(args_num);
                    let block = if block != 0 {
                        Some(MethodRef::from(block))
                    } else {
                        None
                    };
                    self.eval_send(methodref, receiver, args, keyword, block)?;
                    self.pc += 21;
                }
                Inst::SEND_SELF => {
                    let receiver = context.self_value;
                    let method_id = read_id(iseq, self.pc + 1);
                    let args_num = read32(iseq, self.pc + 5) as usize;
                    let kw_args_num = read32(iseq, self.pc + 9) as usize;
                    let cache_slot = read32(iseq, self.pc + 13) as usize;
                    let block = read32(iseq, self.pc + 17);
                    let methodref = self.get_method_from_cache(cache_slot, receiver, method_id)?;

                    let keyword = if kw_args_num != 0 {
                        let val = self.exec_stack.pop().unwrap();
                        eprintln!("{}", self.val_pp(val));
                        Some(val)
                    } else {
                        None
                    };
                    let args = self.pop_args_to_ary(args_num);
                    let block = if block != 0 {
                        Some(MethodRef::from(block))
                    } else {
                        None
                    };
                    self.eval_send(methodref, receiver, args, keyword, block)?;
                    self.pc += 21;
                }
                Inst::DEF_CLASS => {
                    let is_module = read8(iseq, self.pc + 1) == 1;
                    let id = IdentId::from(read32(iseq, self.pc + 2));
                    let methodref = MethodRef::from(read32(iseq, self.pc + 6));
                    let super_val = self.exec_stack.pop().unwrap();
                    let superclass = match super_val.as_class() {
                        Some(class) => class,
                        None => {
                            let val = self.val_pp(super_val);
                            return Err(self.error_type(format!(
                                "Superclass must be a class. (given:{:?})",
                                val
                            )));
                        }
                    };
                    let (val, classref) = match self.globals.object_class.constants.get(&id) {
                        Some(val) => (
                            val.clone(),
                            match val.as_module() {
                                Some(classref) => {
                                    if classref.superclass != Some(superclass) {
                                        eprintln!(
                                            "prev: {:?}",
                                            match classref.superclass {
                                                None => "None",
                                                Some(class) =>
                                                    self.globals.get_ident_name(class.name),
                                            }
                                        );
                                        eprintln!(
                                            " new: {:?}",
                                            self.globals.get_ident_name(superclass.name)
                                        );
                                        return Err(self.error_type(format!(
                                            "superclass mismatch for class {}.",
                                            self.globals.get_ident_name(id),
                                        )));
                                    };
                                    classref
                                }
                                None => {
                                    return Err(self.error_type(format!(
                                        "{} is not a class.",
                                        self.val_pp(val.clone())
                                    )))
                                }
                            },
                        ),
                        None => {
                            let classref = ClassRef::from(id, superclass);
                            let val = if is_module {
                                PackedValue::module(&mut self.globals, classref)
                            } else {
                                PackedValue::class(&mut self.globals, classref)
                            };
                            self.class().constants.insert(id, val);
                            (val, classref)
                        }
                    };

                    self.class_stack.push(classref);

                    self.eval_send(methodref, val, VecArray::new0(), None, None)?;
                    self.pc += 10;
                    self.class_stack.pop().unwrap();
                }
                Inst::DEF_METHOD => {
                    let id = IdentId::from(read32(iseq, self.pc + 1));
                    let methodref = MethodRef::from(read32(iseq, self.pc + 5));
                    if self.class_stack.len() == 0 {
                        // A method defined in "top level" is registered as an object method.
                        self.add_object_method(id, methodref);
                    } else {
                        // A method defined in a class definition is registered as an instance method of the class.
                        let classref = self.class_stack.last().unwrap().clone();
                        self.add_instance_method(classref, id, methodref);
                    }
                    self.exec_stack.push(PackedValue::symbol(id));
                    self.pc += 9;
                }
                Inst::DEF_CLASS_METHOD => {
                    let id = IdentId::from(read32(iseq, self.pc + 1));
                    let methodref = MethodRef::from(read32(iseq, self.pc + 5));
                    if self.class_stack.len() == 0 {
                        // A method defined in "top level" is registered as an object method.
                        self.add_object_method(id, methodref);
                    } else {
                        // A method defined in a class definition is registered as a class method of the class.
                        let classref = self.class_stack.last().unwrap().clone();
                        self.add_class_method(classref, id, methodref);
                    }
                    self.exec_stack.push(PackedValue::symbol(id));
                    self.pc += 9;
                }
                Inst::TO_S => {
                    let val = self.exec_stack.pop().unwrap();
                    let res = PackedValue::string(self.val_to_s(val));
                    self.exec_stack.push(res);
                    self.pc += 1;
                }
                Inst::POP => {
                    self.exec_stack.pop().unwrap();
                    self.pc += 1;
                }
                Inst::DUP => {
                    let len = read32(iseq, self.pc + 1) as usize;
                    let stack_len = self.exec_stack.len();
                    for i in stack_len - len..stack_len {
                        let val = self.exec_stack[i];
                        self.exec_stack.push(val);
                    }
                    self.pc += 5;
                }
                Inst::TAKE => {
                    let len = read32(iseq, self.pc + 1) as usize;
                    let val = self.exec_stack.pop().unwrap();
                    match val.as_object() {
                        Some(obj) => match obj.kind {
                            ObjKind::Array(info) => push_some(self, &info.elements, len),
                            _ => push_one(self, val, len),
                        },
                        None => push_one(self, val, len),
                    }
                    self.pc += 5;

                    fn push_one(vm: &mut VM, val: PackedValue, len: usize) {
                        vm.exec_stack.push(val);
                        for _ in 0..len - 1 {
                            vm.exec_stack.push(PackedValue::nil());
                        }
                    }
                    fn push_some(vm: &mut VM, elem: &Vec<PackedValue>, len: usize) {
                        let ary_len = elem.len();
                        if len <= ary_len {
                            for i in 0..len {
                                vm.exec_stack.push(elem[i]);
                            }
                        } else {
                            for i in 0..ary_len {
                                vm.exec_stack.push(elem[i]);
                            }
                            for _ in ary_len..len {
                                vm.exec_stack.push(PackedValue::nil());
                            }
                        }
                    }
                }
                _ => return Err(self.error_unimplemented("Unimplemented instruction.")),
            }
        }

        fn read_id(iseq: &ISeq, pc: usize) -> IdentId {
            IdentId::from(read32(iseq, pc))
        }

        fn read_lvar_id(iseq: &ISeq, pc: usize) -> LvarId {
            LvarId::from_usize(read32(iseq, pc) as usize)
        }

        fn read64(iseq: &ISeq, pc: usize) -> u64 {
            let ptr = iseq[pc..pc + 1].as_ptr() as *const u64;
            unsafe { *ptr }
        }

        fn read32(iseq: &ISeq, pc: usize) -> u32 {
            let ptr = iseq[pc..pc + 1].as_ptr() as *const u32;
            unsafe { *ptr }
        }

        fn read8(iseq: &ISeq, pc: usize) -> u8 {
            iseq[pc]
        }
    }
}

impl VM {
    pub fn error_nomethod(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::NoMethod(msg.into()),
            self.source_info(),
            loc,
        )
    }
    pub fn error_undefined_method(
        &self,
        method_name: impl Into<String>,
        class_name: impl Into<String>,
    ) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::NoMethod(format!(
                "undefined method `{}' for {}",
                method_name.into(),
                class_name.into()
            )),
            self.source_info(),
            loc,
        )
    }
    pub fn error_unimplemented(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Unimplemented(msg.into()),
            self.source_info(),
            loc,
        )
    }
    pub fn error_internal(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Internal(msg.into()),
            self.source_info(),
            loc,
        )
    }
    pub fn error_name(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Name(msg.into()), self.source_info(), loc)
    }
    pub fn error_type(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Type(msg.into()), self.source_info(), loc)
    }
    pub fn error_argument(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Argument(msg.into()),
            self.source_info(),
            loc,
        )
    }
    pub fn error_index(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Index(msg.into()), self.source_info(), loc)
    }
    pub fn check_args_num(&self, len: usize, min: usize, max: usize) -> Result<(), RubyError> {
        if min <= len && len <= max {
            Ok(())
        } else {
            Err(self.error_argument(format!(
                "Wrong number of arguments. (given {}, expected {}..{})",
                len, min, max
            )))
        }
    }
}

impl VM {
    pub fn val_as_class(&self, val: PackedValue) -> Result<ClassRef, RubyError> {
        match val.as_class() {
            Some(class_ref) => Ok(class_ref),
            None => {
                let val = self.val_pp(val);
                Err(self.error_type(format!("Must be a class. (given:{:?})", val)))
            }
        }
    }

    pub fn val_as_module(&self, val: PackedValue) -> Result<ClassRef, RubyError> {
        match val.as_module() {
            Some(class_ref) => Ok(class_ref),
            None => {
                let val = self.val_pp(val);
                Err(self.error_type(format!("Must be a module/class. (given:{:?})", val)))
            }
        }
    }
}

impl VM {
    fn get_loc(&self) -> Loc {
        let sourcemap = &self.context().iseq_ref.iseq_sourcemap;
        sourcemap
            .iter()
            .find(|x| x.0 == ISeqPos::from_usize(self.pc))
            .unwrap_or(&(ISeqPos::from_usize(0), Loc(0, 0)))
            .1
    }

    fn get_method_from_cache(
        &mut self,
        cache_slot: usize,
        receiver: PackedValue,
        method_id: IdentId,
    ) -> Result<MethodRef, RubyError> {
        let (rec_class, class_method) = match receiver.as_class() {
            Some(cref) => (cref, true),
            None => (receiver.get_class(&self.globals), false),
        };
        match self.globals.get_method_from_cache(cache_slot, receiver) {
            Some(method) => Ok(method),
            _ => {
                let method = if class_method {
                    self.get_class_method(rec_class, method_id)?
                } else {
                    self.get_instance_method(rec_class, method_id)?
                };
                self.globals
                    .set_method_cache_entry(cache_slot, rec_class, class_method, method);
                Ok(method)
            }
        }
    }

    fn get_constant(&self, mut class: ClassRef, id: IdentId) -> Result<PackedValue, RubyError> {
        loop {
            match class.constants.get(&id) {
                Some(val) => {
                    return Ok(val.clone());
                }
                None => match class.superclass {
                    Some(superclass) => {
                        class = superclass;
                    }
                    None => {
                        let name = self.globals.get_ident_name(id).clone();
                        return Err(self.error_name(format!("Uninitialized constant {}.", name)));
                    }
                },
            }
        }
    }
}

impl VM {
    fn eval_add(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<(), RubyError> {
        let val = if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            PackedValue::fixnum(((*rhs as i64) + (*lhs as i64) - 2) / 2)
        } else if rhs.is_packed_num() && lhs.is_packed_num() {
            if rhs.is_packed_fixnum() {
                PackedValue::flonum(rhs.as_packed_fixnum() as f64 + lhs.as_packed_flonum())
            } else if lhs.is_packed_fixnum() {
                PackedValue::flonum(rhs.as_packed_flonum() + lhs.as_packed_fixnum() as f64)
            } else {
                PackedValue::flonum(rhs.as_packed_flonum() + lhs.as_packed_flonum())
            }
        } else {
            match (lhs.unpack(), rhs.unpack()) {
                (Value::FixNum(lhs), Value::FixNum(rhs)) => PackedValue::fixnum(lhs + rhs),
                (Value::FixNum(lhs), Value::FloatNum(rhs)) => PackedValue::flonum(lhs as f64 + rhs),
                (Value::FloatNum(lhs), Value::FixNum(rhs)) => PackedValue::flonum(lhs + rhs as f64),
                (Value::FloatNum(lhs), Value::FloatNum(rhs)) => PackedValue::flonum(lhs + rhs),
                (Value::Object(l_ref), _) => {
                    let method = IdentId::_ADD;
                    match l_ref.get_instance_method(method) {
                        Some(mref) => {
                            self.eval_send(mref.clone(), lhs, VecArray::new1(rhs), None, None)?;
                        }
                        None => {
                            return Err(self.error_undefined_method(
                                format!("+ {}", self.globals.get_class_name(rhs)),
                                self.globals.get_class_name(lhs),
                            ))
                        }
                    };
                    return Ok(());
                }
                (_, _) => {
                    return Err(self.error_undefined_method(
                        format!("+ {}", self.globals.get_class_name(rhs)),
                        self.globals.get_class_name(lhs),
                    ))
                }
            }
        };
        self.exec_stack.push(val);
        Ok(())
    }

    fn eval_addi(&mut self, lhs: PackedValue, i: i32) -> Result<(), RubyError> {
        let val = if lhs.is_packed_fixnum() {
            PackedValue::fixnum(lhs.as_packed_fixnum() + i as i64)
        } else if lhs.is_packed_num() {
            PackedValue::flonum(lhs.as_packed_flonum() + i as f64)
        } else {
            match lhs.unpack() {
                Value::FixNum(lhs) => PackedValue::fixnum(lhs + i as i64),
                Value::FloatNum(lhs) => PackedValue::flonum(lhs + i as f64),
                Value::Object(l_ref) => {
                    let method = IdentId::_ADD;
                    match l_ref.get_instance_method(method) {
                        Some(mref) => {
                            self.eval_send(
                                mref.clone(),
                                lhs,
                                VecArray::new1(PackedValue::fixnum(i as i64)),
                                None,
                                None,
                            )?;
                        }
                        None => {
                            return Err(self.error_undefined_method(
                                format!("+ Integer"),
                                self.globals.get_class_name(lhs),
                            ))
                        }
                    };
                    return Ok(());
                }
                _ => {
                    return Err(self.error_undefined_method(
                        format!("+ Integer"),
                        self.globals.get_class_name(lhs),
                    ))
                }
            }
        };
        self.exec_stack.push(val);
        Ok(())
    }

    fn eval_sub(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<(), RubyError> {
        let val = if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            PackedValue::fixnum(((*lhs as i64) - (*rhs as i64)) / 2)
        } else if lhs.is_packed_num() && rhs.is_packed_num() {
            if lhs.is_packed_fixnum() {
                PackedValue::flonum(lhs.as_packed_fixnum() as f64 - rhs.as_packed_flonum())
            } else if rhs.is_packed_fixnum() {
                PackedValue::flonum(lhs.as_packed_flonum() - rhs.as_packed_fixnum() as f64)
            } else {
                PackedValue::flonum(lhs.as_packed_flonum() - rhs.as_packed_flonum())
            }
        } else {
            match (lhs.unpack(), rhs.unpack()) {
                (Value::FixNum(lhs), Value::FixNum(rhs)) => PackedValue::fixnum(lhs - rhs),
                (Value::FixNum(lhs), Value::FloatNum(rhs)) => PackedValue::flonum(lhs as f64 - rhs),
                (Value::FloatNum(lhs), Value::FixNum(rhs)) => PackedValue::flonum(lhs - rhs as f64),
                (Value::FloatNum(lhs), Value::FloatNum(rhs)) => PackedValue::flonum(lhs - rhs),
                (Value::Object(l_ref), _) => {
                    let method = IdentId::_SUB;
                    match l_ref.get_instance_method(method) {
                        Some(mref) => {
                            self.eval_send(mref.clone(), lhs, VecArray::new1(rhs), None, None)?;
                        }
                        None => {
                            return Err(self.error_undefined_method(
                                format!("- {}", self.globals.get_class_name(rhs)),
                                self.globals.get_class_name(lhs),
                            ))
                        }
                    };
                    return Ok(());
                }
                (_, _) => {
                    return Err(self.error_undefined_method(
                        format!("- {}", self.globals.get_class_name(rhs)),
                        self.globals.get_class_name(lhs),
                    ))
                }
            }
        };
        self.exec_stack.push(val);
        Ok(())
    }

    fn eval_subi(&mut self, lhs: PackedValue, i: i32) -> Result<(), RubyError> {
        let val = if lhs.is_packed_fixnum() {
            PackedValue::fixnum(lhs.as_packed_fixnum() - i as i64)
        } else if lhs.is_packed_num() {
            PackedValue::flonum(lhs.as_packed_flonum() - i as f64)
        } else {
            match lhs.unpack() {
                Value::FixNum(lhs) => PackedValue::fixnum(lhs - i as i64),
                Value::FloatNum(lhs) => PackedValue::flonum(lhs - i as f64),
                Value::Object(l_ref) => {
                    let method = IdentId::_SUB;
                    match l_ref.get_instance_method(method) {
                        Some(mref) => {
                            self.eval_send(
                                mref.clone(),
                                lhs,
                                VecArray::new1(PackedValue::fixnum(i as i64)),
                                None,
                                None,
                            )?;
                        }
                        None => {
                            return Err(self.error_undefined_method(
                                format!("- Integer"),
                                self.globals.get_class_name(lhs),
                            ))
                        }
                    };
                    return Ok(());
                }
                _ => {
                    return Err(self.error_undefined_method(
                        format!("- Integer"),
                        self.globals.get_class_name(lhs),
                    ))
                }
            }
        };
        self.exec_stack.push(val);
        Ok(())
    }

    fn eval_mul(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<(), RubyError> {
        let val = if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            PackedValue::fixnum(lhs.as_packed_fixnum() * rhs.as_packed_fixnum())
        } else if lhs.is_packed_num() && rhs.is_packed_num() {
            if lhs.is_packed_fixnum() {
                PackedValue::flonum(lhs.as_packed_fixnum() as f64 * rhs.as_packed_flonum())
            } else if rhs.is_packed_fixnum() {
                PackedValue::flonum(lhs.as_packed_flonum() * rhs.as_packed_fixnum() as f64)
            } else {
                PackedValue::flonum(lhs.as_packed_flonum() * rhs.as_packed_flonum())
            }
        } else {
            match (lhs.unpack(), rhs.unpack()) {
                (Value::FixNum(lhs), Value::FixNum(rhs)) => PackedValue::fixnum(lhs * rhs),
                (Value::FixNum(lhs), Value::FloatNum(rhs)) => PackedValue::flonum(lhs as f64 * rhs),
                (Value::FloatNum(lhs), Value::FixNum(rhs)) => PackedValue::flonum(lhs * rhs as f64),
                (Value::FloatNum(lhs), Value::FloatNum(rhs)) => PackedValue::flonum(lhs * rhs),
                (Value::Object(l_ref), _) => {
                    let method = IdentId::_MUL;
                    match l_ref.get_instance_method(method) {
                        Some(mref) => {
                            self.eval_send(mref.clone(), lhs, VecArray::new1(rhs), None, None)?;
                        }
                        None => {
                            return Err(self.error_undefined_method(
                                format!("* {}", self.globals.get_class_name(rhs)),
                                self.globals.get_class_name(lhs),
                            ))
                        }
                    };
                    return Ok(());
                }
                (_, _) => {
                    return Err(self.error_undefined_method(
                        format!("* {}", self.globals.get_class_name(rhs)),
                        self.globals.get_class_name(lhs),
                    ))
                }
            }
        };
        self.exec_stack.push(val);
        Ok(())
    }

    fn eval_div(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs / rhs).pack()),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => {
                Ok(Value::FloatNum((lhs as f64) / rhs).pack())
            }
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => {
                Ok(Value::FloatNum(lhs / (rhs as f64)).pack())
            }
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs / rhs).pack()),
            (_, _) => Err(self.error_nomethod("NoMethodError: '/'")),
        }
    }

    fn eval_rem(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs % rhs).pack()),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => {
                Ok(Value::FloatNum((lhs as f64) % rhs).pack())
            }
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => {
                Ok(Value::FloatNum(lhs % (rhs as f64)).pack())
            }
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs % rhs).pack()),
            (_, _) => Err(self.error_nomethod("NoMethodError: '%'")),
        }
    }

    fn eval_shl(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs << rhs)),
            (Value::Object(l_ref), _) => {
                let method = self.globals.get_ident_id("<<");
                match l_ref.get_instance_method(method) {
                    Some(mref) => {
                        self.eval_send(mref.clone(), lhs, VecArray::new1(rhs), None, None)?;
                        Ok(self.exec_stack.pop().unwrap())
                    }
                    None => Err(self.error_undefined_method(
                        format!("<< {}", self.globals.get_class_name(rhs)),
                        self.globals.get_class_name(lhs),
                    )),
                }
            }
            (_, _) => {
                return Err(self.error_undefined_method(
                    format!("<< {}", self.globals.get_class_name(rhs)),
                    self.globals.get_class_name(lhs),
                ))
            }
        }
    }

    fn eval_shr(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs >> rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    fn eval_bitand(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs & rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '&'")),
        }
    }

    fn eval_bitor(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs | rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '|'")),
        }
    }

    fn eval_bitxor(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs ^ rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '^'")),
        }
    }

    fn eval_bitnot(&mut self, lhs: PackedValue) -> VMResult {
        match lhs.unpack() {
            Value::FixNum(lhs) => Ok(PackedValue::fixnum(!lhs)),
            _ => Err(self.error_nomethod("NoMethodError: '~'")),
        }
    }

    pub fn eval_eq(&self, rhs: PackedValue, lhs: PackedValue) -> Result<bool, RubyError> {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(*lhs == *rhs);
        }
        if lhs.is_packed_num() && rhs.is_packed_num() {
            if lhs.is_packed_fixnum() {
                return Ok(lhs.as_packed_fixnum() as f64 == rhs.as_packed_flonum());
            } else if rhs.is_packed_fixnum() {
                return Ok(lhs.as_packed_flonum() == rhs.as_packed_fixnum() as f64);
            } else {
                return Ok(*lhs == *rhs);
            }
        }
        match (&lhs.unpack(), &rhs.unpack()) {
            (Value::Nil, Value::Nil) => Ok(true),
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(lhs == rhs),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(lhs == rhs),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(*lhs as f64 == *rhs),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(*lhs == *rhs as f64),
            (Value::String(lhs), Value::String(rhs)) => Ok(lhs == rhs),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(lhs == rhs),
            (Value::Symbol(lhs), Value::Symbol(rhs)) => Ok(lhs == rhs),
            (Value::Object(lhs), Value::Object(rhs)) => match (&lhs.kind, &rhs.kind) {
                (ObjKind::Ordinary, ObjKind::Ordinary) => Ok(lhs == rhs),
                (ObjKind::Class(lhs), ObjKind::Class(rhs)) => Ok(lhs == rhs),
                (ObjKind::Array(lhs), ObjKind::Array(rhs)) => {
                    let lhs = &lhs.elements;
                    let rhs = &rhs.elements;
                    if lhs.len() != rhs.len() {
                        return Ok(false);
                    }
                    for i in 0..lhs.len() {
                        if !self.eval_eq(lhs[i], rhs[i])? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
                (ObjKind::Range(lhs), ObjKind::Range(rhs)) => {
                    if lhs.start == rhs.start && lhs.end == rhs.end && lhs.exclude == rhs.exclude {
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }
                _ => Err(self.error_nomethod(format!("NoMethodError: {:?} == {:?}", lhs, rhs))),
            },
            _ => Err(self.error_nomethod(format!("NoMethodError: {:?} == {:?}", lhs, rhs))),
        }
    }

    fn eval_ge(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(PackedValue::bool(
                lhs.as_packed_fixnum() >= rhs.as_packed_fixnum(),
            ));
        }
        if lhs.is_packed_num() && rhs.is_packed_num() {
            if lhs.is_packed_fixnum() {
                return Ok(PackedValue::bool(
                    lhs.as_packed_fixnum() as f64 >= rhs.as_packed_flonum(),
                ));
            } else if rhs.is_packed_fixnum() {
                return Ok(PackedValue::bool(
                    lhs.as_packed_flonum() >= rhs.as_packed_fixnum() as f64,
                ));
            } else {
                return Ok(PackedValue::bool(
                    lhs.as_packed_flonum() >= rhs.as_packed_flonum(),
                ));
            }
        }
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::bool(lhs >= rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => {
                Ok(PackedValue::bool(lhs >= (rhs as f64)))
            }
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::bool(lhs as f64 >= rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::bool(lhs >= rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>='")),
        }
    }

    fn eval_gt(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(PackedValue::bool(
                lhs.as_packed_fixnum() > rhs.as_packed_fixnum(),
            ));
        }
        if lhs.is_packed_num() && rhs.is_packed_num() {
            if lhs.is_packed_fixnum() {
                return Ok(PackedValue::bool(
                    lhs.as_packed_fixnum() as f64 > rhs.as_packed_flonum(),
                ));
            } else if rhs.is_packed_fixnum() {
                return Ok(PackedValue::bool(
                    lhs.as_packed_flonum() > rhs.as_packed_fixnum() as f64,
                ));
            } else {
                return Ok(PackedValue::bool(
                    lhs.as_packed_flonum() > rhs.as_packed_flonum(),
                ));
            }
        }
        let b = match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => PackedValue::bool(lhs > rhs),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => PackedValue::bool(lhs > (rhs as f64)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => PackedValue::bool(lhs as f64 > rhs),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => PackedValue::bool(lhs > rhs),
            (_, _) => {
                return Err(self.error_undefined_method(
                    format!("> {}", self.globals.get_class_name(rhs)),
                    self.globals.get_class_name(lhs),
                ))
            }
        };
        Ok(b)
    }
}

impl VM {
    pub fn val_to_bool(&self, val: PackedValue) -> bool {
        !val.is_nil() && !val.is_false_val() && !val.is_uninitialized()
    }

    pub fn val_to_s(&self, val: PackedValue) -> String {
        match val.unpack() {
            Value::Uninitialized => "[Uninitialized]".to_string(),
            Value::Nil => "".to_string(),
            Value::Bool(b) => match b {
                true => "true".to_string(),
                false => "false".to_string(),
            },
            Value::FixNum(i) => i.to_string(),
            Value::FloatNum(f) => f.to_string(),
            Value::String(s) => format!("{}", s),
            Value::Symbol(i) => format!(":{}", self.globals.get_ident_name(i)),

            Value::Char(c) => format!("{:x}", c),
            Value::Object(oref) => match oref.kind {
                ObjKind::Class(cref) => {
                    format! {"Class({})", self.globals.get_ident_name(cref.name)}
                }
                ObjKind::Ordinary => {
                    format! {"Instance({}:{:?})", self.globals.get_ident_name(oref.classref.name), oref}
                }
                ObjKind::Array(aref) => match aref.elements.len() {
                    0 => "[]".to_string(),
                    1 => format!("[{}]", self.val_to_s(aref.elements[0])),
                    len => {
                        let mut result = self.val_to_s(aref.elements[0]);
                        for i in 1..len {
                            result = format!("{}, {}", result, self.val_to_s(aref.elements[i]));
                        }
                        format! {"[{}]", result}
                    }
                },
                ObjKind::Range(rref) => {
                    let start = self.val_to_s(rref.start);
                    let end = self.val_to_s(rref.end);
                    let sym = if rref.exclude { "..." } else { ".." };
                    format!("({}{}{})", start, sym, end)
                }
                _ => format!("{:?}", oref.kind),
            },
        }
    }

    pub fn val_pp(&self, val: PackedValue) -> String {
        match val.unpack() {
            Value::Nil => "nil".to_string(),
            Value::String(s) => format!("\"{}\"", s),
            Value::Object(oref) => match oref.kind {
                ObjKind::Class(cref) => format! {"{}", self.globals.get_ident_name(cref.name)},
                ObjKind::Array(aref) => match aref.elements.len() {
                    0 => "[]".to_string(),
                    1 => format!("[{}]", self.val_pp(aref.elements[0])),
                    len => {
                        let mut result = self.val_pp(aref.elements[0]);
                        for i in 1..len {
                            result = format!("{}, {}", result, self.val_pp(aref.elements[i]));
                        }
                        format! {"[{}]", result}
                    }
                },
                ObjKind::Hash(href) => match href.map.len() {
                    0 => "{}".to_string(),
                    _ => {
                        let mut result = "".to_string();
                        let mut first = true;
                        for (k, v) in &href.map {
                            result = if first {
                                format!("{} => {}", self.val_pp(k.clone()), self.val_pp(v.clone()))
                            } else {
                                format!(
                                    "{}, {} => {}",
                                    result,
                                    self.val_pp(k.clone()),
                                    self.val_pp(v.clone())
                                )
                            };
                            first = false;
                        }
                        format! {"{{{}}}", result}
                    }
                },
                ObjKind::Ordinary => {
                    format! {"#<{}:{:?}>", self.globals.get_ident_name(oref.classref.name), oref}
                }
                _ => self.val_to_s(val),
            },
            _ => self.val_to_s(val),
        }
    }
}

impl VM {
    pub fn eval_send(
        &mut self,
        methodref: MethodRef,
        receiver: PackedValue,
        args: VecArray,
        keyword: Option<PackedValue>,
        block: Option<MethodRef>,
    ) -> Result<(), RubyError> {
        let info = self.globals.get_method_info(methodref);
        #[allow(unused_variables, unused_mut)]
        let mut inst: u8;
        #[cfg(feature = "perf")]
        {
            inst = self.perf.get_prev_inst();
        }
        let val = match info {
            MethodInfo::BuiltinFunc { func, .. } => {
                #[cfg(feature = "perf")]
                {
                    self.perf.get_perf(Perf::EXTERN);
                }
                let val = func(self, receiver, args, block)?;
                #[cfg(feature = "perf")]
                {
                    self.perf.get_perf_no_count(inst);
                }
                val
            }
            MethodInfo::AttrReader { id } => match receiver.unpack() {
                Value::Object(oref) => match oref.instance_var.get(id) {
                    Some(v) => v.clone(),
                    None => PackedValue::nil(),
                },
                _ => unreachable!("AttrReader must be used only for class instance."),
            },
            MethodInfo::AttrWriter { id } => match receiver.unpack() {
                Value::Object(mut oref) => {
                    oref.instance_var.insert(*id, args[0]);
                    args[0]
                }
                _ => unreachable!("AttrReader must be used only for class instance."),
            },
            MethodInfo::RubyFunc { iseq } => {
                let iseq = iseq.clone();
                self.vm_run(receiver, iseq, None, args, keyword, block)?;
                #[cfg(feature = "perf")]
                {
                    self.perf.get_perf_no_count(inst);
                }
                return Ok(());
            }
        };
        self.exec_stack.push(val);
        Ok(())
    }
}

impl VM {
    pub fn add_class_method(
        &mut self,
        mut class: ClassRef,
        id: IdentId,
        info: MethodRef,
    ) -> Option<MethodRef> {
        self.globals.class_version += 1;
        class.class_method.insert(id, info)
    }

    pub fn add_instance_method(
        &mut self,
        mut class: ClassRef,
        id: IdentId,
        info: MethodRef,
    ) -> Option<MethodRef> {
        self.globals.class_version += 1;
        class.instance_method.insert(id, info)
    }

    pub fn get_class_method(
        &self,
        class: ClassRef,
        method: IdentId,
    ) -> Result<MethodRef, RubyError> {
        match class.get_class_method(method) {
            Some(methodref) => Ok(*methodref),
            None => self.get_instance_method(self.globals.class_class, method),
        }
    }

    pub fn get_instance_method(
        &self,
        classref: ClassRef,
        method: IdentId,
    ) -> Result<MethodRef, RubyError> {
        let mut class = classref;
        loop {
            match class.get_instance_method(method) {
                Some(methodref) => return Ok(*methodref),
                None => match class.superclass {
                    Some(superclass) => class = superclass,
                    None => {
                        let method_name = self.globals.get_ident_name(method);
                        let class_name = self.globals.get_ident_name(classref.name);
                        return Err(self.error_undefined_method(method_name, class_name));
                    }
                },
            };
        }
    }

    pub fn get_object_method(&self, method: IdentId) -> Result<MethodRef, RubyError> {
        match self.globals.get_object_method(method) {
            Some(info) => Ok(info.clone()),
            None => return Err(self.error_unimplemented("Method not defined.")),
        }
    }

    pub fn add_object_method(&mut self, id: IdentId, info: MethodRef) {
        self.add_instance_method(self.globals.object_class, id, info);
    }
}

impl VM {
    fn get_array_index(&self, idx_arg: i64, len: usize) -> Result<usize, RubyError> {
        if idx_arg < 0 {
            let i = len as i64 + idx_arg;
            if i < 0 {
                return Err(self.error_unimplemented("Index out of range."));
            };
            Ok(i as usize)
        } else if idx_arg < len as i64 {
            Ok(idx_arg as usize)
        } else {
            return Err(self.error_unimplemented("Index out of range."));
        }
    }

    fn pop_args(&mut self, arg_num: usize) -> Vec<PackedValue> {
        let mut args = vec![];
        for _ in 0..arg_num {
            let val = self.exec_stack.pop().unwrap();
            match val.as_splat() {
                Some(ary) => {
                    for elem in &ary.elements {
                        args.push(elem.clone());
                    }
                }
                None => {
                    args.push(val);
                }
            };
        }
        args
    }

    fn pop_key_value_pair(&mut self, arg_num: usize) -> HashMap<PackedValue, PackedValue> {
        let mut hash = HashMap::new();
        for _ in 0..arg_num {
            let value = self.exec_stack.pop().unwrap();
            let key = self.exec_stack.pop().unwrap();
            hash.insert(key, value);
        }
        hash
    }

    fn pop_args_to_ary(&mut self, arg_num: usize) -> VecArray {
        let mut args = VecArray::new(0);
        for _ in 0..arg_num {
            let val = self.exec_stack.pop().unwrap();
            match val.as_splat() {
                Some(ary) => {
                    for elem in &ary.elements {
                        args.push(elem.clone());
                    }
                }
                None => args.push(val),
            };
        }
        args
    }

    fn create_proc_obj(&mut self, method: MethodRef) -> Result<PackedValue, RubyError> {
        let context = self.context_stack.last_mut().unwrap();
        if context.on_stack {
            *context = ContextRef::new(context.dup_context());
            context.on_stack = false;
        }
        let context = self.create_context_from_method(method)?;
        Ok(PackedValue::procobj(&self.globals, context))
    }

    fn create_context_from_method(&mut self, method: MethodRef) -> Result<ContextRef, RubyError> {
        let iseq = self.globals.get_method_info(method).as_iseq(&self)?;
        let outer = self.context();
        Ok(ContextRef::from(outer.self_value, None, iseq, Some(outer)))
    }
}
