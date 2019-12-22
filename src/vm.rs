mod array;
mod builtin;
mod class;
mod codegen;
mod context;
mod globals;
mod method;
mod object;
#[cfg(feature = "perf")]
mod perf;
mod procobj;
mod range;
pub mod value;
mod vm_inst;

use crate::error::{RubyError, RuntimeErrKind};
use crate::node::*;
pub use crate::parser::{LvarCollector, LvarId};
pub use crate::util::*;
pub use array::*;
pub use builtin::*;
pub use class::*;
use codegen::{Codegen, ISeq, ISeqPos};
pub use context::*;
pub use globals::*;
pub use method::*;
pub use object::*;
#[cfg(feature = "perf")]
use perf::*;
pub use procobj::*;
pub use range::*;
use std::collections::HashMap;
pub use value::*;
use vm_inst::*;

pub type ValueTable = HashMap<IdentId, PackedValue>;

pub type VMResult = Result<PackedValue, RubyError>;

#[derive(Debug, Clone)]
pub struct VM {
    // Global info
    pub globals: Globals,
    pub const_table: ValueTable,
    pub codegen: Codegen,
    // VM state
    pub context_stack: Vec<ContextRef>,
    pub class_stack: Vec<ClassRef>,
    pub exec_stack: Vec<PackedValue>,
    pub pc: usize,
    #[cfg(feature = "perf")]
    perf: Perf,
}

impl VM {
    pub fn new(ident_table: Option<IdentifierTable>) -> Self {
        let mut globals = Globals::new(ident_table);
        let mut const_table = HashMap::new();
        const_table.insert(
            globals.get_ident_id("Object"),
            PackedValue::class(&globals, globals.object_class),
        );
        const_table.insert(
            globals.get_ident_id("Class"),
            PackedValue::class(&globals, globals.class_class),
        );
        const_table.insert(
            globals.get_ident_id("Array"),
            PackedValue::class(&globals, globals.array_class),
        );
        const_table.insert(
            globals.get_ident_id("Proc"),
            PackedValue::class(&globals, globals.proc_class),
        );
        VM {
            globals,
            const_table,
            codegen: Codegen::new(),
            class_stack: vec![],
            context_stack: vec![],
            exec_stack: vec![],
            pc: 0,
            #[cfg(feature = "perf")]
            perf: Perf::new(),
        }
    }

    pub fn init(&mut self, ident_table: IdentifierTable) {
        self.globals.ident_table = ident_table;
    }

    pub fn context(&self) -> ContextRef {
        *self.context_stack.last().unwrap()
    }

    /// Get local variable table.
    pub fn get_outer_context(&mut self, outer: u32) -> ContextRef {
        let mut context = self.context();
        for _ in 0..outer {
            context = context.outer.unwrap();
        }
        context
    }

    pub fn run(&mut self, node: &Node, lvar_collector: &LvarCollector) -> VMResult {
        #[cfg(feature = "perf")]
        {
            self.perf.set_prev_inst(Perf::CODEGEN);
        }
        let methodref = self.codegen.gen_iseq(
            &mut self.globals,
            &vec![],
            node,
            lvar_collector,
            true,
            false,
        )?;
        let iseq = self.globals.get_method_info(methodref).as_iseq(&self)?;
        let class = self.globals.main_class;
        let main_object = PackedValue::class(&mut self.globals, class);
        self.vm_run(main_object, iseq, None, vec![], 0)?;
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

    pub fn run_repl(
        &mut self,
        node: &Node,
        lvar_collector: &LvarCollector,
        mut context: ContextRef,
    ) -> VMResult {
        #[cfg(feature = "perf")]
        {
            self.perf.set_prev_inst(Perf::CODEGEN);
        }
        let methodref = self.codegen.gen_iseq(
            &mut self.globals,
            &vec![],
            node,
            lvar_collector,
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

    pub fn vm_run(
        &mut self,
        self_value: PackedValue,
        iseq: ISeqRef,
        outer: Option<ContextRef>,
        args: Vec<PackedValue>,
        block: u32,
    ) -> Result<(), RubyError> {
        let mut context = Context::new(self_value, block, iseq, outer);
        context.set_arguments(args);
        if block != 0 {
            if let Some(id) = iseq.lvar.block_param() {
                let val = self.create_proc_obj(MethodRef::from(block))?;
                *context.get_mut_lvar(id) = val;
            }
        }
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
                Inst::END => {
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
                Inst::CONCAT_STRING => {
                    let rhs = self.exec_stack.pop().unwrap().as_string();
                    let lhs = self.exec_stack.pop().unwrap().as_string();
                    let val = match (lhs, rhs) {
                        (Some(lhs), Some(rhs)) => Value::String(format!("{}{}", lhs, rhs)),
                        (_, _) => unreachable!("Illegal CAONCAT_STRING arguments."),
                    };
                    self.exec_stack.push(val.pack());
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
                Inst::SET_CONST => {
                    let id = read_id(iseq, self.pc + 1);
                    let val = self.exec_stack.pop().unwrap();
                    self.const_table.insert(id, val);
                    self.pc += 5;
                }
                Inst::GET_CONST => {
                    let id = read_id(iseq, self.pc + 1);
                    match self.const_table.get(&id) {
                        Some(val) => self.exec_stack.push(val.clone()),
                        None => {
                            let name = self.globals.get_ident_name(id).clone();
                            return Err(
                                self.error_name(format!("uninitialized constant {}.", name))
                            );
                        }
                    }
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
                    match self.exec_stack.pop().unwrap().as_array() {
                        Some(mut aref) => {
                            let index = if args[0].is_packed_fixnum() {
                                args[0].as_packed_fixnum()
                            } else {
                                return Err(self.error_unimplemented("Index must be an integer."));
                            };
                            let index = self.get_array_index(index, aref.elements.len())?;
                            let val = self.exec_stack.pop().unwrap();
                            aref.elements[index] = val;
                        }
                        None => {
                            return Err(self.error_unimplemented(
                                "Currently, []= is supported only for array.",
                            ))
                        }
                    }
                    self.pc += 5;
                }
                Inst::GET_ARRAY_ELEM => {
                    let arg_num = read32(iseq, self.pc + 1) as usize;
                    let args = self.pop_args(arg_num);
                    match self.exec_stack.pop().unwrap().as_array() {
                        Some(aref) => {
                            let index = match args[0].as_fixnum() {
                                Some(num) => num,
                                None => {
                                    return Err(
                                        self.error_type("No implicit conversion into Integer")
                                    )
                                }
                            };
                            let index = self.get_array_index(index, aref.elements.len())?;
                            if arg_num == 1 {
                                let elem = aref.elements[index];
                                self.exec_stack.push(elem);
                            } else {
                                let len = match args[1].as_fixnum() {
                                    Some(num) => num,
                                    None => {
                                        return Err(
                                            self.error_type("No implicit conversion into Integer")
                                        )
                                    }
                                };
                                if len < 0 {
                                    self.exec_stack.push(PackedValue::nil());
                                } else {
                                    let len = len as usize;
                                    let end = std::cmp::min(aref.elements.len(), index + len);
                                    let ary = (&aref.elements[index..end]).to_vec();
                                    let ary_object =
                                        PackedValue::array(&self.globals, ArrayRef::from(ary));
                                    self.exec_stack.push(ary_object);
                                }
                            };
                        }
                        None => {
                            return Err(self
                                .error_unimplemented("Currently, [] is supported only for array."))
                        }
                    }
                    self.pc += 5;
                }
                Inst::CREATE_RANGE => {
                    let start = self.exec_stack.pop().unwrap();
                    let end = self.exec_stack.pop().unwrap();
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
                    let cache_slot = read32(iseq, self.pc + 9) as usize;
                    let block = read32(iseq, self.pc + 13);
                    let methodref = self.get_method_from_cache(cache_slot, receiver, method_id)?;

                    let args = self.pop_args(args_num);

                    self.eval_send(methodref, receiver, args, block)?;
                    self.pc += 17;
                }
                Inst::SEND_SELF => {
                    let receiver = context.self_value;
                    let method_id = read_id(iseq, self.pc + 1);
                    let args_num = read32(iseq, self.pc + 5) as usize;
                    let cache_slot = read32(iseq, self.pc + 9) as usize;
                    let block = read32(iseq, self.pc + 13);
                    let methodref = self.get_method_from_cache(cache_slot, receiver, method_id)?;

                    let args = self.pop_args(args_num);

                    self.eval_send(methodref, receiver, args, block)?;
                    self.pc += 17;
                }
                Inst::DEF_CLASS => {
                    let id = IdentId::from(read32(iseq, self.pc + 1));
                    let methodref = MethodRef::from(read32(iseq, self.pc + 5));
                    let super_val = self.exec_stack.pop().unwrap();
                    let superclass = match super_val.as_class() {
                        Some(cref) => cref,
                        None => {
                            return Err(self.error_type(format!(
                                "{} is not a class.",
                                self.val_pp(super_val.clone())
                            )))
                        }
                    };
                    let (val, classref) = match self.const_table.get(&id) {
                        Some(val) => (
                            val.clone(),
                            match val.as_class() {
                                Some(classref) => {
                                    if classref.superclass != Some(superclass) {
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
                            let val = PackedValue::class(&mut self.globals, classref);
                            self.const_table.insert(id, val);
                            (val, classref)
                        }
                    };

                    self.class_stack.push(classref);

                    self.eval_send(methodref, val, vec![], 0)?;
                    self.pc += 9;
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
                    self.exec_stack.push(PackedValue::nil());
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
                    self.exec_stack.push(PackedValue::nil());
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
    }
}

impl VM {
    pub fn error_nomethod(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::NoMethod(msg.into()), loc)
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
            loc,
        )
    }
    pub fn error_unimplemented(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Unimplemented(msg.into()), loc)
    }
    pub fn error_internal(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Internal(msg.into()), loc)
    }
    pub fn error_name(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Name(msg.into()), loc)
    }
    pub fn error_type(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Type(msg.into()), loc)
    }

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
                            self.eval_send(mref.clone(), lhs, vec![rhs], 0)?;
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
                                vec![PackedValue::fixnum(i as i64)],
                                0,
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
                            self.eval_send(mref.clone(), lhs, vec![rhs], 0)?;
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
                        format!("+ {}", self.globals.get_class_name(rhs)),
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
                                vec![PackedValue::fixnum(i as i64)],
                                0,
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
                            self.eval_send(mref.clone(), lhs, vec![rhs], 0)?;
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
            (_, _) => Err(self.error_nomethod("NoMethodError: '*'")),
        }
    }

    fn eval_shl(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs << rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '<<'")),
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
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    fn eval_bitor(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs | rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    fn eval_bitxor(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs ^ rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    pub fn eval_eq(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<bool, RubyError> {
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
        !val.is_nil() && !val.is_false_val()
    }

    pub fn val_to_s(&self, val: PackedValue) -> String {
        match val.unpack() {
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
                ObjKind::Class(cref) => format! {"Class({})", self.globals.get_ident_name(cref.id)},
                ObjKind::Ordinary => {
                    format! {"Instance({}:{:?})", self.globals.get_ident_name(oref.classref.id), oref}
                }
                ObjKind::Array(aref) => match aref.elements.len() {
                    0 => "[]".to_string(),
                    1 => format!("[{}]", self.val_to_s(aref.elements[0])),
                    len => {
                        let mut result = self.val_to_s(aref.elements[0]);
                        for i in 1..len {
                            result = format!("{},{}", result, self.val_to_s(aref.elements[i]));
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
                ObjKind::Class(cref) => format! {"{}", self.globals.get_ident_name(cref.id)},
                ObjKind::Ordinary => {
                    format! {"#<{}:{:?}>", self.globals.get_ident_name(oref.classref.id), oref}
                }
                _ => self.val_to_s(val),
            },
            _ => self.val_to_s(val),
        }
    }
}

impl VM {
    pub fn init_builtin(&mut self) {
        builtin::Builtin::init_builtin(&mut self.globals);
    }

    pub fn eval_send(
        &mut self,
        methodref: MethodRef,
        receiver: PackedValue,
        args: Vec<PackedValue>,
        block: u32,
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
                let val = func(self, receiver, args)?;
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
                //args.push(block);
                self.vm_run(receiver, iseq, None, args, block)?;
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
                        let class_name = self.globals.get_ident_name(classref.id);
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
            args.push(self.exec_stack.pop().unwrap());
        }
        args.reverse();
        args
    }

    fn create_proc_obj(&mut self, method: MethodRef) -> Result<PackedValue, RubyError> {
        let iseq = self.globals.get_method_info(method).as_iseq(&self)?;
        let context = self.context_stack.last_mut().unwrap();
        if context.on_stack {
            *context = ContextRef::new(context.dup_context());
            context.on_stack = false;
        }
        let outer = self.context();
        let context = ContextRef::from(outer.self_value, 0, iseq, Some(outer));
        Ok(PackedValue::procobj(&self.globals, context))
    }
}
