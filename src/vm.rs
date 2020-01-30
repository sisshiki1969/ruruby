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
mod regexp;
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
pub use object::*;
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
    pub global_var: ValueTable,
    // VM state
    pub context_stack: Vec<ContextRef>,
    pub class_stack: Vec<PackedValue>,
    pub exec_stack: Vec<PackedValue>,
    pub pc: usize,
    #[cfg(feature = "perf")]
    perf: Perf,
}

impl VM {
    pub fn new() -> Self {
        let mut globals = Globals::new();

        macro_rules! set_builtin_class {
            ($name:expr, $class_object:ident) => {
                let id = globals.get_ident_id($name);
                globals.object.set_var(id, globals.$class_object);
            };
        }

        set_builtin_class!("Object", object);
        set_builtin_class!("Module", module);
        set_builtin_class!("Class", class);
        set_builtin_class!("Integer", integer);
        set_builtin_class!("Array", array);
        set_builtin_class!("Proc", procobj);
        set_builtin_class!("Range", range);
        set_builtin_class!("String", string);
        set_builtin_class!("Hash", hash);
        set_builtin_class!("Method", method);
        set_builtin_class!("Regexp", regexp);

        let id = globals.get_ident_id("StandardError");
        let class = PackedValue::class(&globals, globals.class_class);
        globals.object.set_var(id, class);

        let mut vm = VM {
            globals,
            root_path: vec![],
            global_var: HashMap::new(),
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

    pub fn classref(&self) -> ClassRef {
        if self.class_stack.len() == 0 {
            self.globals.object_class
        } else {
            self.class_stack.last().unwrap().as_module().unwrap()
        }
    }

    pub fn class(&self) -> PackedValue {
        if self.class_stack.len() == 0 {
            self.globals.object
        } else {
            *self.class_stack.last().unwrap()
        }
    }

    pub fn stack_push(&mut self, val: PackedValue) {
        self.exec_stack.push(val)
    }

    pub fn stack_pop(&mut self) -> PackedValue {
        self.exec_stack.pop().unwrap()
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
        self.vm_run(
            self.globals.main_object,
            iseq,
            None,
            VecArray::new0(),
            None,
            None,
        )?;
        let val = self.stack_pop();
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
        let val = self.stack_pop();
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
                    self.stack_push(PackedValue::nil());
                    self.pc += 1;
                }
                Inst::PUSH_TRUE => {
                    self.stack_push(PackedValue::true_val());
                    self.pc += 1;
                }
                Inst::PUSH_FALSE => {
                    self.stack_push(PackedValue::false_val());
                    self.pc += 1;
                }
                Inst::PUSH_SELF => {
                    self.stack_push(context.self_value);
                    self.pc += 1;
                }
                Inst::PUSH_FIXNUM => {
                    let num = read64(iseq, self.pc + 1);
                    self.pc += 9;
                    self.stack_push(PackedValue::fixnum(num as i64));
                }
                Inst::PUSH_FLONUM => {
                    let num = unsafe { std::mem::transmute(read64(iseq, self.pc + 1)) };
                    self.pc += 9;
                    self.stack_push(PackedValue::flonum(num));
                }
                Inst::PUSH_STRING => {
                    let id = read_id(iseq, self.pc + 1);
                    let string = self.globals.get_ident_name(id).to_string();
                    self.stack_push(PackedValue::string(string));
                    self.pc += 5;
                }
                Inst::PUSH_SYMBOL => {
                    let id = read_id(iseq, self.pc + 1);
                    self.stack_push(PackedValue::symbol(id));
                    self.pc += 5;
                }
                Inst::ADD => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    self.eval_add(lhs, rhs)?;
                    self.pc += 1;
                }
                Inst::ADDI => {
                    let lhs = self.stack_pop();
                    let i = read32(iseq, self.pc + 1) as i32;
                    self.eval_addi(lhs, i)?;
                    self.pc += 5;
                }
                Inst::SUB => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    self.eval_sub(lhs, rhs)?;
                    self.pc += 1;
                }
                Inst::SUBI => {
                    let lhs = self.stack_pop();
                    let i = read32(iseq, self.pc + 1) as i32;
                    self.eval_subi(lhs, i)?;
                    self.pc += 5;
                }
                Inst::MUL => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    self.eval_mul(lhs, rhs)?;
                    self.pc += 1;
                }
                Inst::POW => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    self.eval_exp(lhs, rhs)?;
                    self.pc += 1;
                }
                Inst::DIV => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_div(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::REM => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_rem(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SHR => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_shr(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SHL => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_shl(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::BIT_AND => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_bitand(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::BIT_OR => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_bitor(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::BIT_XOR => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_bitxor(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::BIT_NOT => {
                    let lhs = self.stack_pop();
                    let val = self.eval_bitnot(lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::EQ => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = PackedValue::bool(self.eval_eq(lhs, rhs)?);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NE => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = PackedValue::bool(!self.eval_eq(lhs, rhs)?);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::TEQ => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let res = match lhs.is_class() {
                        Some(_) if rhs.get_class_object(&self.globals).id() == lhs.id() => true,
                        _ => match self.eval_eq(lhs, rhs) {
                            Ok(res) => res,
                            Err(_) => false,
                        },
                    };
                    let val = PackedValue::bool(res);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GT => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_gt(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GE => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_ge(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NOT => {
                    let lhs = self.stack_pop();
                    let val = PackedValue::bool(!self.val_to_bool(lhs));
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::CONCAT_STRING => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = match (lhs.as_string(), rhs.as_string()) {
                        (Some(lhs), Some(rhs)) => PackedValue::string(format!("{}{}", lhs, rhs)),
                        (_, _) => unreachable!("Illegal CAONCAT_STRING arguments."),
                    };
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SET_LOCAL => {
                    let id = read_lvar_id(iseq, self.pc + 1);
                    let outer = read32(iseq, self.pc + 5);
                    let val = self.stack_pop();
                    let mut cref = self.get_outer_context(outer);
                    *cref.get_mut_lvar(id) = val;
                    self.pc += 9;
                }
                Inst::GET_LOCAL => {
                    let id = read_lvar_id(iseq, self.pc + 1);
                    let outer = read32(iseq, self.pc + 5);
                    let cref = self.get_outer_context(outer);
                    let val = cref.get_lvar(id);
                    self.stack_push(val);
                    self.pc += 9;
                }
                Inst::CHECK_LOCAL => {
                    let id = read_lvar_id(iseq, self.pc + 1);
                    let outer = read32(iseq, self.pc + 5);
                    let cref = self.get_outer_context(outer);
                    let val = cref.get_lvar(id).is_uninitialized();
                    self.stack_push(PackedValue::bool(val));
                    self.pc += 9;
                }
                Inst::SET_CONST => {
                    let id = read_id(iseq, self.pc + 1);
                    let val = self.stack_pop();
                    self.class().set_var(id, val);
                    self.pc += 5;
                }
                Inst::GET_CONST => {
                    let id = read_id(iseq, self.pc + 1);
                    let val = match self.get_env_const(id) {
                        Some(val) => val,
                        None => self.get_super_const(self.class(), id)?,
                    };
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::GET_CONST_TOP => {
                    let id = read_id(iseq, self.pc + 1);
                    let class = self.globals.object;
                    let val = self.get_super_const(class, id)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::GET_SCOPE => {
                    let parent = self.stack_pop();
                    let id = read_id(iseq, self.pc + 1);
                    let val = self.get_super_const(parent, id)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_INSTANCE_VAR => {
                    let var_id = read_id(iseq, self.pc + 1);
                    let mut self_obj = self.context().self_value.as_object();
                    let new_val = self.stack_pop();
                    self_obj.set_var(var_id, new_val);
                    self.pc += 5;
                }
                Inst::GET_INSTANCE_VAR => {
                    let var_id = read_id(iseq, self.pc + 1);
                    let self_obj = self.context().self_value.as_object();
                    let val = match self_obj.get_var(var_id) {
                        Some(val) => val.clone(),
                        None => PackedValue::nil(),
                    };
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_GLOBAL_VAR => {
                    let var_id = read_id(iseq, self.pc + 1);
                    let new_val = self.stack_pop();
                    self.set_global_var(var_id, new_val);
                    self.pc += 5;
                }
                Inst::GET_GLOBAL_VAR => {
                    let var_id = read_id(iseq, self.pc + 1);
                    let val = self.get_global_var(var_id);
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_ARRAY_ELEM => {
                    let arg_num = read32(iseq, self.pc + 1) as usize;
                    let args = self.pop_args(arg_num);
                    let arg_num = args.len();
                    match self.stack_pop().is_object() {
                        Some(oref) => {
                            match oref.kind {
                                ObjKind::Array(mut aref) => {
                                    self.check_args_num(arg_num, 1, 2)?;
                                    let index = args[0].expect_fixnum(&self, "Index")?;
                                    if arg_num == 1 && index >= aref.elements.len() as i64 {
                                        let padding = index as usize - aref.elements.len();
                                        aref.elements
                                            .append(&mut vec![PackedValue::nil(); padding]);
                                        aref.elements.push(self.stack_pop());
                                    } else {
                                        let index =
                                            self.get_array_index(index, aref.elements.len())?;
                                        let val = self.stack_pop();
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
                                    let val = self.stack_pop();
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
                    match self.stack_pop().unpack() {
                        Value::Object(oref) => {
                            match oref.kind {
                                ObjKind::Array(aref) => {
                                    self.check_args_num(arg_num, 1, 2)?;
                                    let index = args[0].expect_fixnum(&self, "Index")?;
                                    let index = self.get_array_index(index, aref.elements.len())?;
                                    if arg_num == 1 {
                                        let elem = aref.elements[index];
                                        self.stack_push(elem);
                                    } else {
                                        let len = args[1].expect_fixnum(&self, "Index")?;
                                        if len < 0 {
                                            self.stack_push(PackedValue::nil());
                                        } else {
                                            let len = len as usize;
                                            let end =
                                                std::cmp::min(aref.elements.len(), index + len);
                                            let ary = (&aref.elements[index..end]).to_vec();
                                            let ary_object =
                                                PackedValue::array_from(&self.globals, ary);
                                            self.stack_push(ary_object);
                                        }
                                    };
                                }
                                ObjKind::Hash(href) => {
                                    self.check_args_num(arg_num, 1, 2)?;
                                    let key = href.map.keys().find(|x| x.equal(args[0]));
                                    let val = match key {
                                        Some(key) => match href.map.get(key) {
                                            Some(val) => val.clone(),
                                            None => PackedValue::nil(),
                                        },
                                        None => PackedValue::nil(),
                                    };
                                    self.stack_push(val);
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
                            self.stack_push(PackedValue::fixnum(val));
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
                    let array = self.stack_pop();
                    let res = match array.as_array() {
                        Some(array) => PackedValue::splat(&self.globals, array),
                        None => array,
                    };
                    self.stack_push(res);
                    self.pc += 1;
                }
                Inst::CREATE_RANGE => {
                    let start = self.stack_pop();
                    let end = self.stack_pop();
                    if !start.is_packed_fixnum() || !end.is_packed_fixnum() {
                        return Err(self.error_argument("Bad value for range."));
                    };
                    let exclude_val = self.stack_pop();
                    let exclude_end = self.val_to_bool(exclude_val);
                    let range = PackedValue::range(&self.globals, start, end, exclude_end);
                    self.stack_push(range);
                    self.pc += 1;
                }
                Inst::CREATE_ARRAY => {
                    let arg_num = read32(iseq, self.pc + 1) as usize;
                    let elems = self.pop_args(arg_num);
                    let array = PackedValue::array_from(&self.globals, elems);
                    self.stack_push(array);
                    self.pc += 5;
                }
                Inst::CREATE_PROC => {
                    let method = MethodRef::from(read32(iseq, self.pc + 1));
                    let proc_obj = self.create_proc_obj(method)?;
                    self.stack_push(proc_obj);
                    self.pc += 5;
                }
                Inst::CREATE_HASH => {
                    let arg_num = read32(iseq, self.pc + 1) as usize;
                    let key_value = self.pop_key_value_pair(arg_num);
                    let hash = PackedValue::hash(&self.globals, HashRef::from(key_value));
                    self.stack_push(hash);
                    self.pc += 5;
                }
                Inst::CREATE_REGEXP => {
                    let arg = self.stack_pop();
                    let mut arg = match arg.as_string() {
                        Some(arg) => arg.clone(),
                        None => {
                            return Err(self.error_argument("Illegal argument for CREATE_REGEXP"))
                        }
                    };
                    match arg.pop().unwrap() {
                        'i' => arg.insert_str(0, "(?i)"),
                        'm' => arg.insert_str(0, "(?m)"),
                        'x' => arg.insert_str(0, "(?x)"),
                        'o' => arg.insert_str(0, "(?o)"),
                        _ => {}
                    };
                    let regexpref = match regexp::RegexpRef::from_string(&arg) {
                        Ok(regex) => regex,
                        Err(err) => {
                            return Err(self.error_argument(format!(
                                "Illegal regular expression: {:?}\n/{}/",
                                err, arg
                            )))
                        }
                    };
                    let regexp = PackedValue::regexp(&self.globals, regexpref);
                    self.stack_push(regexp);
                    self.pc += 1;
                }
                Inst::JMP => {
                    let disp = read32(iseq, self.pc + 1) as i32 as i64;
                    self.pc = ((self.pc as i64) + 5 + disp) as usize;
                }
                Inst::JMP_IF_FALSE => {
                    let val = self.stack_pop();
                    if self.val_to_bool(val) {
                        self.pc += 5;
                    } else {
                        let disp = read32(iseq, self.pc + 1) as i32 as i64;
                        self.pc = ((self.pc as i64) + 5 + disp) as usize;
                    }
                }
                Inst::SEND => {
                    let receiver = self.stack_pop();
                    let method_id = read_id(iseq, self.pc + 1);
                    let args_num = read32(iseq, self.pc + 5) as usize;
                    let kw_args_num = read32(iseq, self.pc + 9) as usize;
                    let cache_slot = read32(iseq, self.pc + 13) as usize;
                    let block = read32(iseq, self.pc + 17);
                    let methodref = self.get_method_from_cache(cache_slot, receiver, method_id)?;

                    let keyword = if kw_args_num != 0 {
                        let val = self.stack_pop();
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
                        let val = self.stack_pop();
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
                    let super_val = self.stack_pop();
                    let val = match self.globals.object.get_var(id) {
                        Some(val) => {
                            let classref = self.val_as_module(val.clone())?;
                            if !super_val.is_nil() && classref.superclass.id() != super_val.id() {
                                return Err(self.error_type(format!(
                                    "superclass mismatch for class {}.",
                                    self.globals.get_ident_name(id),
                                )));
                            };
                            val.clone()
                        }
                        None => {
                            let super_val = if !super_val.is_nil() {
                                if super_val.is_class().is_none() {
                                    let val = self.val_pp(super_val);
                                    return Err(self.error_type(format!(
                                        "Superclass must be a class. (given:{:?})",
                                        val
                                    )));
                                };
                                super_val
                            } else {
                                self.globals.object
                            };
                            let classref = ClassRef::from(id, super_val);
                            let val = if is_module {
                                PackedValue::module(&mut self.globals, classref)
                            } else {
                                PackedValue::class(&mut self.globals, classref)
                            };
                            self.class().set_var(id, val);
                            val
                        }
                    };

                    self.class_stack.push(val);
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
                        self.add_instance_method(self.class(), id, methodref);
                    }
                    self.stack_push(PackedValue::symbol(id));
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
                        self.add_singleton_method(self.class(), id, methodref)?;
                    }
                    self.stack_push(PackedValue::symbol(id));
                    self.pc += 9;
                }
                Inst::TO_S => {
                    let val = self.stack_pop();
                    let res = PackedValue::string(self.val_to_s(val));
                    self.stack_push(res);
                    self.pc += 1;
                }
                Inst::POP => {
                    self.stack_pop();
                    self.pc += 1;
                }
                Inst::DUP => {
                    let len = read32(iseq, self.pc + 1) as usize;
                    let stack_len = self.exec_stack.len();
                    for i in stack_len - len..stack_len {
                        let val = self.exec_stack[i];
                        self.stack_push(val);
                    }
                    self.pc += 5;
                }
                Inst::TAKE => {
                    let len = read32(iseq, self.pc + 1) as usize;
                    let val = self.stack_pop();
                    match val.is_object() {
                        Some(obj) => match obj.kind {
                            ObjKind::Array(info) => push_some(self, &info.elements, len),
                            _ => push_one(self, val, len),
                        },
                        None => push_one(self, val, len),
                    }
                    self.pc += 5;

                    fn push_one(vm: &mut VM, val: PackedValue, len: usize) {
                        vm.stack_push(val);
                        for _ in 0..len - 1 {
                            vm.stack_push(PackedValue::nil());
                        }
                    }
                    fn push_some(vm: &mut VM, elem: &Vec<PackedValue>, len: usize) {
                        let ary_len = elem.len();
                        if len <= ary_len {
                            for i in 0..len {
                                vm.stack_push(elem[i]);
                            }
                        } else {
                            for i in 0..ary_len {
                                vm.stack_push(elem[i]);
                            }
                            for _ in ary_len..len {
                                vm.stack_push(PackedValue::nil());
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
    pub fn error_undefined_op(
        &self,
        method_name: impl Into<String>,
        rhs: PackedValue,
        lhs: PackedValue,
    ) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::NoMethod(format!(
                "undefined method `{}' {} for {}",
                method_name.into(),
                self.globals.get_class_name(rhs),
                self.globals.get_class_name(lhs)
            )),
            self.source_info(),
            loc,
        )
    }
    pub fn error_undefined_method(
        &self,
        method_name: impl Into<String>,
        receiver: PackedValue,
    ) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::NoMethod(format!(
                "undefined method `{}' for {}",
                method_name.into(),
                self.globals.get_class_name(receiver)
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
        match val.is_class() {
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
        let rec_class = receiver.get_class_object_for_method(&self.globals);
        match self.globals.get_method_from_cache(cache_slot, rec_class) {
            Some(method) => Ok(method),
            _ => {
                /*
                eprintln!(
                    "cache miss! {} {}",
                    self.val_pp(receiver),
                    self.globals.get_ident_name(method_id)
                );*/
                let method = self.get_instance_method(rec_class, method_id)?;
                self.globals
                    .set_method_cache_entry(cache_slot, rec_class, method);
                Ok(method)
            }
        }
    }

    // Search class stack for the constant.
    fn get_env_const(&self, id: IdentId) -> Option<PackedValue> {
        for class in self.class_stack.iter().rev() {
            match class.get_var(id) {
                Some(val) => return Some(val.clone()),
                None => {}
            }
        }
        None
    }

    // Search class inheritance chain for the constant.
    fn get_super_const(
        &self,
        mut class: PackedValue,
        id: IdentId,
    ) -> Result<PackedValue, RubyError> {
        loop {
            match class.get_var(id) {
                Some(val) => {
                    return Ok(val.clone());
                }
                None => match class.superclass() {
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

    fn get_global_var(&self, id: IdentId) -> PackedValue {
        match self.global_var.get(&id) {
            Some(val) => val.clone(),
            None => PackedValue::nil(),
        }
    }

    fn set_global_var(&mut self, id: IdentId, val: PackedValue) {
        self.global_var.insert(id, val);
    }
}

impl VM {
    fn fallback_to_method(
        &mut self,
        method: IdentId,
        lhs: PackedValue,
        rhs: PackedValue,
        l_ref: ObjectRef,
    ) -> Result<(), RubyError> {
        match l_ref.get_instance_method(method) {
            Some(mref) => {
                self.eval_send(mref.clone(), lhs, VecArray::new1(rhs), None, None)?;
                Ok(())
            }
            None => {
                let name = self.globals.get_ident_name(method);
                Err(self.error_undefined_op(name, rhs, lhs))
            }
        }
    }

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
            match lhs.is_object() {
                Some(oref) => return self.fallback_to_method(IdentId::_ADD, lhs, rhs, oref),
                None => match (lhs.unpack(), rhs.unpack()) {
                    (Value::FixNum(lhs), Value::FixNum(rhs)) => PackedValue::fixnum(lhs + rhs),
                    (Value::FixNum(lhs), Value::FloatNum(rhs)) => {
                        PackedValue::flonum(lhs as f64 + rhs)
                    }
                    (Value::FloatNum(lhs), Value::FixNum(rhs)) => {
                        PackedValue::flonum(lhs + rhs as f64)
                    }
                    (Value::FloatNum(lhs), Value::FloatNum(rhs)) => PackedValue::flonum(lhs + rhs),
                    (_, _) => return Err(self.error_undefined_op("+", rhs, lhs)),
                },
            }
        };
        self.stack_push(val);
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
                    return self.fallback_to_method(
                        IdentId::_ADD,
                        lhs,
                        PackedValue::fixnum(i as i64),
                        l_ref.as_ref(),
                    )
                }
                _ => return Err(self.error_undefined_op("+", PackedValue::fixnum(i as i64), lhs)),
            }
        };
        self.stack_push(val);
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
                    return self.fallback_to_method(IdentId::_SUB, lhs, rhs, l_ref.as_ref())
                }
                (_, _) => return Err(self.error_undefined_op("-", rhs, lhs)),
            }
        };
        self.stack_push(val);
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
                    return self.fallback_to_method(
                        IdentId::_SUB,
                        lhs,
                        PackedValue::fixnum(i as i64),
                        l_ref.as_ref(),
                    )
                }
                _ => return Err(self.error_undefined_op("-", PackedValue::fixnum(i as i64), lhs)),
            }
        };
        self.stack_push(val);
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
            match lhs.is_object() {
                Some(oref) => return self.fallback_to_method(IdentId::_MUL, lhs, rhs, oref),
                None => match (lhs.unpack(), rhs.unpack()) {
                    (Value::FixNum(lhs), Value::FixNum(rhs)) => PackedValue::fixnum(lhs * rhs),
                    (Value::FixNum(lhs), Value::FloatNum(rhs)) => {
                        PackedValue::flonum(lhs as f64 * rhs)
                    }
                    (Value::FloatNum(lhs), Value::FixNum(rhs)) => {
                        PackedValue::flonum(lhs * rhs as f64)
                    }
                    (Value::FloatNum(lhs), Value::FloatNum(rhs)) => PackedValue::flonum(lhs * rhs),
                    (_, _) => return Err(self.error_undefined_op("*", rhs, lhs)),
                },
            }
        };
        self.stack_push(val);
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
            (_, _) => return Err(self.error_undefined_op("/", rhs, lhs)),
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
            (_, _) => return Err(self.error_undefined_op("%", rhs, lhs)),
        }
    }

    fn eval_exp(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<(), RubyError> {
        let val = if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            PackedValue::flonum((lhs.as_packed_fixnum() as f64).powf(rhs.as_packed_fixnum() as f64))
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
                (Value::FixNum(lhs), Value::FixNum(rhs)) => {
                    PackedValue::flonum((lhs as f64).powf(rhs as f64))
                }
                (Value::FixNum(lhs), Value::FloatNum(rhs)) => {
                    PackedValue::flonum((lhs as f64).powf(rhs))
                }
                (Value::FloatNum(lhs), Value::FixNum(rhs)) => {
                    PackedValue::flonum(lhs.powf(rhs as f64))
                }
                (Value::FloatNum(lhs), Value::FloatNum(rhs)) => PackedValue::flonum(lhs.powf(rhs)),
                (Value::Object(l_ref), _) => {
                    let method = IdentId::_POW;
                    match l_ref.as_ref().get_instance_method(method) {
                        Some(mref) => {
                            self.eval_send(mref.clone(), lhs, VecArray::new1(rhs), None, None)?;
                        }
                        None => return Err(self.error_undefined_op("**", rhs, lhs)),
                    };
                    return Ok(());
                }
                (_, _) => return Err(self.error_undefined_op("**", rhs, lhs)),
            }
        };
        self.stack_push(val);
        Ok(())
    }

    fn eval_shl(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs << rhs)),
            (Value::Object(l_ref), _) => {
                let method = self.globals.get_ident_id("<<");
                match l_ref.as_ref().get_instance_method(method) {
                    Some(mref) => {
                        self.eval_send(mref.clone(), lhs, VecArray::new1(rhs), None, None)?;
                        Ok(self.stack_pop())
                    }
                    None => return Err(self.error_undefined_op("<<", rhs, lhs)),
                }
            }
            (_, _) => return Err(self.error_undefined_op("<<", rhs, lhs)),
        }
    }

    fn eval_shr(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs >> rhs)),
            (_, _) => return Err(self.error_undefined_op(">>", rhs, lhs)),
        }
    }

    fn eval_bitand(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs & rhs)),
            (_, _) => return Err(self.error_undefined_op("&", rhs, lhs)),
        }
    }

    fn eval_bitor(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs | rhs)),
            (_, _) => return Err(self.error_undefined_op("|", rhs, lhs)),
        }
    }

    fn eval_bitxor(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs ^ rhs)),
            (_, _) => return Err(self.error_undefined_op("^", rhs, lhs)),
        }
    }

    fn eval_bitnot(&mut self, lhs: PackedValue) -> VMResult {
        match lhs.unpack() {
            Value::FixNum(lhs) => Ok(PackedValue::fixnum(!lhs)),
            _ => Err(self.error_nomethod("NoMethodError: '~'")),
        }
    }

    pub fn eval_eq(&self, rhs: PackedValue, lhs: PackedValue) -> Result<bool, RubyError> {
        Ok(rhs.equal(lhs))
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
            (_, _) => return Err(self.error_undefined_op(">", rhs, lhs)),
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
            Value::Symbol(i) => format!("{}", self.globals.get_ident_name(i)),
            Value::Char(c) => format!("{:x}", c),
            Value::Object(oref) => match oref.kind {
                ObjKind::Class(cref) => self.globals.get_ident_name(cref.name).to_string(),
                ObjKind::Ordinary => {
                    format! {"#<{}:{:?}>", self.globals.get_ident_name(oref.as_ref().search_class().as_class().name), oref}
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
                ObjKind::Range(rinfo) => {
                    let start = self.val_to_s(rinfo.start);
                    let end = self.val_to_s(rinfo.end);
                    let sym = if rinfo.exclude { "..." } else { ".." };
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
                ObjKind::Class(cref) => match cref.name {
                    Some(id) => format! {"{}", self.globals.get_ident_name(id)},
                    None => format! {"#<Class:0x{:x}>", cref.id()},
                },
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
                    format! {"#<{}:0x{:x}>", self.globals.get_ident_name(oref.as_ref().search_class().as_class().name), oref.as_ref().id()}
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
            MethodInfo::AttrReader { id } => match receiver.is_object() {
                Some(oref) => match oref.get_var(*id) {
                    Some(v) => v.clone(),
                    None => PackedValue::nil(),
                },
                None => unreachable!("AttrReader must be used only for class instance."),
            },
            MethodInfo::AttrWriter { id } => match receiver.is_object() {
                Some(mut oref) => {
                    oref.set_var(*id, args[0]);
                    args[0]
                }
                None => unreachable!("AttrReader must be used only for class instance."),
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
        self.stack_push(val);
        Ok(())
    }
}

impl VM {
    pub fn add_singleton_method(
        &mut self,
        obj: PackedValue,
        id: IdentId,
        info: MethodRef,
    ) -> Result<(), RubyError> {
        self.globals.class_version += 1;
        let singleton = self.get_singleton_class(obj)?;
        let mut singleton_class = singleton.as_class();
        singleton_class.method_table.insert(id, info);
        Ok(())
    }

    pub fn add_instance_method(
        &mut self,
        obj: PackedValue,
        id: IdentId,
        info: MethodRef,
    ) -> Option<MethodRef> {
        self.globals.class_version += 1;
        obj.as_module().unwrap().method_table.insert(id, info)
    }

    pub fn get_instance_method(
        &self,
        mut class: PackedValue,
        method: IdentId,
    ) -> Result<MethodRef, RubyError> {
        loop {
            match class.get_instance_method(method) {
                Some(methodref) => return Ok(methodref),
                None => match class.superclass() {
                    Some(superclass) => class = superclass,
                    None => {
                        let method_name = self.globals.get_ident_name(method);
                        let class_name = self.globals.get_ident_name(class.as_class().name);
                        return Err(self.error_nomethod(format!(
                            "no method `{}' found for {}",
                            method_name, class_name
                        )));
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
        self.add_instance_method(self.globals.object, id, info);
    }

    pub fn get_singleton_class(&mut self, obj: PackedValue) -> VMResult {
        match self.globals.get_singleton_class(obj) {
            Ok(val) => Ok(val),
            Err(()) => Err(self.error_type("Can not define singleton.")),
        }
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
            let val = self.stack_pop();
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
            let value = self.stack_pop();
            let key = self.stack_pop();
            hash.insert(key, value);
        }
        hash
    }

    fn pop_args_to_ary(&mut self, arg_num: usize) -> VecArray {
        let mut args = VecArray::new(0);
        for _ in 0..arg_num {
            let val = self.stack_pop();
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
