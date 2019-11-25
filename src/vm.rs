mod array;
mod builtin;
mod class;
mod codegen;
mod globals;
mod method;
mod object;
#[cfg(feature = "perf")]
mod perf;
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
use codegen::*;
pub use globals::*;
pub use method::*;
pub use object::*;
#[cfg(feature = "perf")]
use perf::*;
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
    pub context_stack: Vec<Context>,
    pub class_stack: Vec<ClassRef>,
    pub exec_stack: Vec<PackedValue>,
    pub pc: usize,
    #[cfg(feature = "perf")]
    perf: Perf,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub self_value: PackedValue,
    pub lvar_scope: Vec<PackedValue>,
    pub iseq_ref: ISeqRef,
    pub methodref: MethodRef,
}

impl Context {
    pub fn new(
        lvar_num: usize,
        self_value: PackedValue,
        iseq_ref: ISeqRef,
        methodref: MethodRef,
    ) -> Self {
        Context {
            self_value,
            lvar_scope: vec![PackedValue::nil(); lvar_num],
            iseq_ref,
            methodref,
        }
    }
}

impl VM {
    pub fn new(ident_table: Option<IdentifierTable>) -> Self {
        let mut globals = Globals::new(ident_table);
        let mut const_table = HashMap::new();
        const_table.insert(
            globals.get_ident_id("Object"),
            PackedValue::class(globals.object_class),
        );
        const_table.insert(
            globals.get_ident_id("Class"),
            PackedValue::class(globals.class_class),
        );
        const_table.insert(
            globals.get_ident_id("Array"),
            PackedValue::class(globals.array_class),
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

    /// Get local variable table.
    pub fn lvar_mut(&mut self, id: LvarId) -> &mut PackedValue {
        &mut self.context_stack.last_mut().unwrap().lvar_scope[id.as_usize()]
    }

    pub fn run(&mut self, node: &Node, lvar_collector: &LvarCollector) -> VMResult {
        #[cfg(feature = "perf")]
        {
            self.perf.set_prev_inst(Perf::CODEGEN);
        }
        let (methodref, iseq) = self
            .codegen
            .gen_iseq(&mut self.globals, node, lvar_collector)?;
        let main_object = PackedValue::class(self.globals.main_class);
        self.context_stack
            .push(Context::new(64, main_object, iseq, methodref));
        let val = self.vm_run()?;
        self.context_stack.pop().unwrap();
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

    pub fn run_repl(&mut self, node: &Node, lvar_collector: &LvarCollector) -> VMResult {
        #[cfg(feature = "perf")]
        {
            self.perf.set_prev_inst(Perf::CODEGEN);
        }
        let (methodref, iseq) = self
            .codegen
            .gen_iseq(&mut self.globals, node, lvar_collector)?;
        if self.context_stack.len() == 0 {
            let main_object = PackedValue::class(self.globals.main_class);
            self.context_stack
                .push(Context::new(64, main_object, iseq, methodref));
        } else {
            self.context_stack.last_mut().unwrap().iseq_ref = iseq;
        }
        let val = self.vm_run()?;
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

    pub fn vm_run(&mut self) -> VMResult {
        let iseq = &*self.context_stack.last().unwrap().iseq_ref.clone();
        let mut pc = 0;
        loop {
            #[cfg(feature = "perf")]
            {
                self.perf.get_perf(iseq[pc]);
            }
            self.pc = pc;
            #[cfg(feature = "trace")]
            {
                println!("{}", Inst::inst_name(iseq[pc]));
            }
            match iseq[pc] {
                Inst::END => match self.exec_stack.pop() {
                    Some(v) => {
                        #[cfg(feature = "perf")]
                        {
                            self.perf.get_perf(Perf::INVALID);
                        }
                        return Ok(v);
                    }
                    None => panic!("Illegal exec stack length."),
                },
                Inst::PUSH_NIL => {
                    self.exec_stack.push(PackedValue::nil());
                    pc += 1;
                }
                Inst::PUSH_TRUE => {
                    self.exec_stack.push(PackedValue::true_val());
                    pc += 1;
                }
                Inst::PUSH_FALSE => {
                    self.exec_stack.push(PackedValue::false_val());
                    pc += 1;
                }
                Inst::PUSH_SELF => {
                    let self_value = self.context_stack.last().unwrap().self_value.clone();
                    self.exec_stack.push(self_value);
                    pc += 1;
                }
                Inst::PUSH_FIXNUM => {
                    let num = read64(iseq, pc + 1);
                    pc += 9;
                    self.exec_stack.push(PackedValue::fixnum(num as i64));
                }
                Inst::PUSH_FLONUM => {
                    let num = unsafe { std::mem::transmute(read64(iseq, pc + 1)) };
                    pc += 9;
                    self.exec_stack.push(PackedValue::flonum(num));
                }
                Inst::PUSH_STRING => {
                    let id = read_id(iseq, pc);
                    let string = self.globals.get_ident_name(id).clone();
                    self.exec_stack.push(PackedValue::string(string));
                    pc += 5;
                }
                Inst::PUSH_SYMBOL => {
                    let id = read_id(iseq, pc);
                    self.exec_stack.push(PackedValue::symbol(id));
                    pc += 5;
                }

                Inst::ADD => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_add(lhs, rhs)?;
                    self.exec_stack.push(val);
                    pc += 1;
                }
                Inst::ADDI => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = read64(iseq, pc + 1) as i64;
                    let val = self.eval_addi(rhs, lhs)?;
                    pc += 9;
                    self.exec_stack.push(val);
                }
                Inst::SUB => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_sub(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::SUBI => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = read64(iseq, pc + 1) as i64;
                    let val = self.eval_subi(rhs, lhs)?;
                    pc += 9;
                    self.exec_stack.push(val);
                }
                Inst::MUL => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_mul(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::DIV => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_div(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::SHR => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_shr(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::SHL => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_shl(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::BIT_AND => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_bitand(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::BIT_OR => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_bitor(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::BIT_XOR => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_bitxor(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::EQ => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = PackedValue::bool(self.eval_eq(lhs, rhs)?);
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::NE => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = PackedValue::bool(!self.eval_eq(lhs, rhs)?);
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::GT => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_gt(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::GE => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_ge(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::CONCAT_STRING => {
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let val = match (lhs, rhs) {
                        (Value::String(lhs), Value::String(rhs)) => {
                            Value::String(format!("{}{}", lhs, rhs))
                        }
                        (_, _) => unreachable!("Illegal CAONCAT_STRING arguments."),
                    };
                    pc += 1;
                    self.exec_stack.push(val.pack());
                }
                Inst::SET_LOCAL => {
                    let id = read_lvar_id(iseq, pc);
                    let val = self.exec_stack.last().unwrap().clone();
                    *self.lvar_mut(id) = val;
                    pc += 5;
                }
                Inst::GET_LOCAL => {
                    let id = read_lvar_id(iseq, pc);
                    let val = self.lvar_mut(id).clone();
                    self.exec_stack.push(val);
                    pc += 5;
                }
                Inst::SET_CONST => {
                    let id = read_id(iseq, pc);
                    let val = self.exec_stack.last().unwrap();
                    self.const_table.insert(id, *val);
                    pc += 5;
                }
                Inst::GET_CONST => {
                    let id = read_id(iseq, pc);
                    match self.const_table.get(&id) {
                        Some(val) => self.exec_stack.push(val.clone()),
                        None => {
                            let name = self.globals.get_ident_name(id).clone();
                            return Err(
                                self.error_name(format!("uninitialized constant {}.", name))
                            );
                        }
                    }
                    pc += 5;
                }
                Inst::SET_INSTANCE_VAR => {
                    let symbol = self.exec_stack.pop().unwrap();
                    let var_id = if symbol.is_packed_symbol() {
                        symbol.as_packed_symbol()
                    } else {
                        unreachable!("SET_INSTANCE_VAR#Illegal instance symbol value.");
                    };
                    let self_var = &self.context_stack.last().unwrap().self_value.unpack();
                    let new_val = self.exec_stack.last().unwrap();
                    match self_var {
                        Value::Instance(id) => id.clone().instance_var.insert(var_id, *new_val),
                        Value::Class(id) => id.clone().instance_var.insert(var_id, *new_val),
                        _ => unreachable!(),
                    };
                    pc += 1;
                }
                Inst::GET_INSTANCE_VAR => {
                    let var_id = read_id(iseq, pc);
                    let self_var = &self.context_stack.last().unwrap().self_value.unpack();
                    let val = match self_var {
                        Value::Instance(id) => id.instance_var.get(&var_id),
                        Value::Class(id) => id.instance_var.get(&var_id),
                        _ => unreachable!(),
                    };
                    let val = match val {
                        Some(val) => val.clone(),
                        None => PackedValue::nil(),
                    };
                    self.exec_stack.push(val);
                    pc += 5;
                }
                Inst::SET_ARRAY_ELEM => {
                    let arg_num = read32(iseq, pc + 1) as usize;
                    let args = self.pop_args(arg_num);
                    match self.exec_stack.pop().unwrap().as_array() {
                        Some(mut aref) => {
                            let index = if args[0].is_packed_fixnum() {
                                args[0].as_packed_fixnum()
                            } else {
                                return Err(self.error_unimplemented("Index must be an integer."));
                            };
                            let index = self.get_array_index(index, aref.elements.len())?;
                            let val = self.exec_stack.last().unwrap();
                            aref.elements[index] = val.clone();
                        }
                        None => {
                            return Err(self.error_unimplemented(
                                "Currently, []= is supported only for array.",
                            ))
                        }
                    }
                    pc += 5;
                }
                Inst::GET_ARRAY_ELEM => {
                    let arg_num = read32(iseq, pc + 1) as usize;
                    let args = self.pop_args(arg_num);
                    match self.exec_stack.pop().unwrap().as_array() {
                        Some(aref) => {
                            let index = if args[0].is_packed_fixnum() {
                                args[0].as_packed_fixnum()
                            } else {
                                return Err(self.error_unimplemented("Index must be an integer."));
                            };
                            let index = self.get_array_index(index, aref.elements.len())?;
                            let elem = aref.elements[index];
                            self.exec_stack.push(elem);
                        }
                        None => {
                            return Err(self
                                .error_unimplemented("Currently, [] is supported only for array."))
                        }
                    }
                    pc += 5;
                }
                Inst::CREATE_RANGE => {
                    let start = self.exec_stack.pop().unwrap();
                    let end = self.exec_stack.pop().unwrap();
                    let exclude = self.exec_stack.pop().unwrap();
                    let range = PackedValue::range(start, end, self.val_to_bool(exclude));
                    self.exec_stack.push(range);
                    pc += 1;
                }
                Inst::CREATE_ARRAY => {
                    let arg_num = read32(iseq, pc + 1) as usize;
                    let elems = self.pop_args(arg_num);
                    let array = PackedValue::array(ArrayRef::from(elems));
                    self.exec_stack.push(array);
                    pc += 5;
                }
                Inst::JMP => {
                    let disp = read32(iseq, pc + 1) as i32 as i64;
                    pc = ((pc as i64) + 5 + disp) as usize;
                }
                Inst::JMP_IF_FALSE => {
                    let val = self.exec_stack.pop().unwrap();
                    if self.val_to_bool(val) {
                        pc += 5;
                    } else {
                        let disp = read32(iseq, pc + 1) as i32 as i64;
                        pc = ((pc as i64) + 5 + disp) as usize;
                    }
                }
                Inst::SEND => {
                    let receiver = self.exec_stack.pop().unwrap();
                    let method_id = read_id(iseq, pc);
                    let methodref = match receiver.unpack() {
                        Value::Nil | Value::FixNum(_) => self.get_toplevel_method(method_id)?,
                        Value::Class(cref) => self.get_class_method(cref, method_id)?,
                        Value::Instance(iref) => {
                            self.get_instance_method(iref.classref, method_id)?
                        }
                        Value::Array(_) => {
                            self.get_instance_method(self.globals.array_class, method_id)?
                        }
                        _ => {
                            return Err(self.error_unimplemented("Unimplemented type of receiver."))
                        }
                    };
                    let args_num = read32(iseq, pc + 5) as usize;
                    let args = self.pop_args(args_num);
                    let val = self.eval_send(methodref, receiver, args)?;
                    self.exec_stack.push(val);
                    pc += 9;
                }
                Inst::DEF_CLASS => {
                    let id = IdentId::from(read32(iseq, pc + 1));
                    let methodref = MethodRef::from(read32(iseq, pc + 5));

                    let classref = ClassRef::from(id, self.globals.object_class);
                    let val = PackedValue::class(classref);
                    self.const_table.insert(id, val);

                    self.class_stack.push(classref);
                    let _ = self.eval_send(methodref, val, vec![])?;
                    self.class_stack.pop().unwrap();

                    self.exec_stack.push(PackedValue::nil());
                    pc += 9;
                }
                Inst::DEF_METHOD => {
                    let id = IdentId::from(read32(iseq, pc + 1));
                    let methodref = MethodRef::from(read32(iseq, pc + 5));
                    //let info = self.globals.get_method_info(methodref).clone();
                    if self.class_stack.len() == 0 {
                        // A method defined in "top level" is registered to the global method table.
                        self.globals.add_toplevel_method(id, methodref);
                    } else {
                        // A method defined in a class definition is registered as a instance method of the class.
                        let mut classref = self.class_stack.last().unwrap().clone();
                        classref.add_instance_method(id, methodref);
                    }
                    self.exec_stack.push(PackedValue::nil());
                    pc += 9;
                }
                Inst::DEF_CLASS_METHOD => {
                    let id = IdentId::from(read32(iseq, pc + 1));
                    let methodref = MethodRef::from(read32(iseq, pc + 5));
                    if self.class_stack.len() == 0 {
                        // A method defined in "top level" is registered to the global method table.
                        self.globals.add_toplevel_method(id, methodref);
                    } else {
                        // A method defined in a class definition is registered as a class method of the class.
                        let mut classref = self.class_stack.last().unwrap().clone();
                        classref.add_class_method(id, methodref);
                    }
                    self.exec_stack.push(PackedValue::nil());
                    pc += 9;
                }
                Inst::TO_S => {
                    let val = self.exec_stack.pop().unwrap();
                    let res = PackedValue::string(self.val_to_s(val));
                    self.exec_stack.push(res);
                    pc += 1;
                }
                Inst::POP => {
                    self.exec_stack.pop().unwrap();
                    pc += 1;
                }
                Inst::DUP => {
                    let len = read32(iseq, pc + 1) as usize;
                    let stack_len = self.exec_stack.len();
                    for i in stack_len - len..stack_len {
                        let val = self.exec_stack[i];
                        self.exec_stack.push(val);
                    }
                    pc += 5;
                }
                _ => return Err(self.error_unimplemented("Unimplemented instruction.")),
            }
        }

        fn read_id(iseq: &ISeq, pc: usize) -> IdentId {
            IdentId::from(read32(iseq, pc + 1))
        }

        fn read_lvar_id(iseq: &ISeq, pc: usize) -> LvarId {
            LvarId::from_usize(read32(iseq, pc + 1) as usize)
        }

        fn read64(iseq: &ISeq, pc: usize) -> u64 {
            let mut num: u64 = (iseq[pc] as u64) << 56;
            num += (iseq[pc + 1] as u64) << 48;
            num += (iseq[pc + 2] as u64) << 40;
            num += (iseq[pc + 3] as u64) << 32;
            num += (iseq[pc + 4] as u64) << 24;
            num += (iseq[pc + 5] as u64) << 16;
            num += (iseq[pc + 6] as u64) << 8;
            num += iseq[pc + 7] as u64;
            num
        }

        fn read32(iseq: &ISeq, pc: usize) -> u32 {
            let mut num: u32 = (iseq[pc] as u32) << 24;
            num += (iseq[pc + 1] as u32) << 16;
            num += (iseq[pc + 2] as u32) << 8;
            num += iseq[pc + 3] as u32;
            num
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
    pub fn error_name(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Name(msg.into()), loc)
    }

    fn get_loc(&self) -> Loc {
        let method = self.context_stack.last().unwrap().methodref;
        let sourcemap = match self.globals.get_method_info(method) {
            MethodInfo::RubyFunc { iseq_sourcemap, .. } => iseq_sourcemap,
            _ => unreachable!("Illegal method_info."),
        };
        sourcemap
            .iter()
            .find(|x| x.0 == ISeqPos::from_usize(self.pc))
            .unwrap_or(&(ISeqPos::from_usize(0), Loc(0, 0)))
            .1
    }
}

impl VM {
    fn eval_add(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(PackedValue::fixnum(((*rhs as i64) + (*lhs as i64) - 2) / 2));
        };
        if rhs.is_packed_num() && lhs.is_packed_num() {
            if rhs.is_packed_fixnum() {
                return Ok(PackedValue::flonum(
                    rhs.as_packed_fixnum() as f64 + lhs.as_packed_flonum(),
                ));
            } else if lhs.is_packed_fixnum() {
                return Ok(PackedValue::flonum(
                    rhs.as_packed_flonum() + lhs.as_packed_fixnum() as f64,
                ));
            } else {
                return Ok(PackedValue::flonum(
                    rhs.as_packed_flonum() + lhs.as_packed_flonum(),
                ));
            }
        }
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs + rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::flonum(lhs as f64 + rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::flonum(lhs + rhs as f64)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::flonum(lhs + rhs)),
            (Value::Instance(l_ref), _) => {
                let method = self.globals.get_ident_id("@add");
                match l_ref.get_instance_method(method) {
                    Some(mref) => self.eval_send(mref.clone(), lhs, vec![rhs]),
                    None => Err(self.error_undefined_method("+", self.globals.get_class_name(lhs))),
                }
            }
            (_, _) => Err(self.error_undefined_method("+", self.globals.get_class_name(lhs))),
        }
    }

    fn eval_addi(&mut self, rhs: i64, lhs: PackedValue) -> VMResult {
        if lhs.is_packed_fixnum() {
            return Ok(PackedValue::fixnum(lhs.as_packed_fixnum() + rhs));
        };
        if lhs.is_packed_num() {
            return Ok(PackedValue::flonum(lhs.as_packed_flonum() + rhs as f64));
        };
        match lhs.unpack() {
            Value::FixNum(lhs) => Ok(PackedValue::fixnum(lhs + rhs)),
            Value::FloatNum(lhs) => Ok(PackedValue::flonum(lhs + rhs as f64)),
            Value::Instance(l_ref) => {
                let method = self.globals.get_ident_id("@add");
                match l_ref.get_instance_method(method) {
                    Some(mref) => self.eval_send(mref.clone(), lhs, vec![PackedValue::fixnum(rhs)]),
                    None => Err(self.error_nomethod("'+'")),
                }
            }
            _ => Err(self.error_nomethod("'+'")),
        }
    }

    fn eval_sub(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(PackedValue::fixnum(((*lhs as i64) - (*rhs as i64)) / 2));
        };
        if lhs.is_packed_num() && rhs.is_packed_num() {
            if lhs.is_packed_fixnum() {
                return Ok(PackedValue::flonum(
                    lhs.as_packed_fixnum() as f64 - rhs.as_packed_flonum(),
                ));
            } else if rhs.is_packed_fixnum() {
                return Ok(PackedValue::flonum(
                    lhs.as_packed_flonum() - rhs.as_packed_fixnum() as f64,
                ));
            } else {
                return Ok(PackedValue::flonum(
                    lhs.as_packed_flonum() - rhs.as_packed_flonum(),
                ));
            }
        }
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs - rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::flonum(lhs as f64 - rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::flonum(lhs - rhs as f64)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::flonum(lhs - rhs)),
            (Value::Instance(l_ref), _) => {
                let method = self.globals.get_ident_id("@sub");
                match l_ref.get_instance_method(method) {
                    Some(mref) => self.eval_send(mref.clone(), lhs, vec![rhs]),
                    None => Err(self.error_nomethod("'-'")),
                }
            }
            (_, _) => Err(self.error_nomethod("'-'")),
        }
    }

    fn eval_subi(&mut self, rhs: i64, lhs: PackedValue) -> VMResult {
        if lhs.is_packed_fixnum() {
            return Ok(PackedValue::fixnum(lhs.as_packed_fixnum() - rhs));
        };
        if lhs.is_packed_num() {
            return Ok(PackedValue::flonum(lhs.as_packed_flonum() - rhs as f64));
        };
        match lhs.unpack() {
            Value::FixNum(lhs) => Ok(PackedValue::fixnum(lhs - rhs)),
            Value::FloatNum(lhs) => Ok(PackedValue::flonum(lhs - rhs as f64)),
            Value::Instance(l_ref) => {
                let method = self.globals.get_ident_id("@sub");
                match l_ref.get_instance_method(method) {
                    Some(mref) => self.eval_send(mref.clone(), lhs, vec![PackedValue::fixnum(rhs)]),
                    None => Err(self.error_nomethod("'-'")),
                }
            }
            _ => Err(self.error_nomethod("'-'")),
        }
    }

    fn eval_mul(&mut self, rhs: PackedValue, lhs: PackedValue) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(PackedValue::fixnum(
                lhs.as_packed_fixnum() * rhs.as_packed_fixnum(),
            ));
        };
        if lhs.is_packed_num() && rhs.is_packed_num() {
            if lhs.is_packed_fixnum() {
                return Ok(PackedValue::flonum(
                    lhs.as_packed_fixnum() as f64 * rhs.as_packed_flonum(),
                ));
            } else if rhs.is_packed_fixnum() {
                return Ok(PackedValue::flonum(
                    lhs.as_packed_flonum() * rhs.as_packed_fixnum() as f64,
                ));
            } else {
                return Ok(PackedValue::flonum(
                    lhs.as_packed_flonum() * rhs.as_packed_flonum(),
                ));
            }
        }
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs * rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::flonum(lhs as f64 * rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::flonum(lhs * rhs as f64)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::flonum(lhs * rhs)),
            (Value::Instance(l_ref), _) => {
                let method = self.globals.get_ident_id("@mul");
                match l_ref.get_instance_method(method) {
                    Some(mref) => self.eval_send(mref.clone(), lhs, vec![rhs]),
                    None => Err(self.error_nomethod("'*'")),
                }
            }
            (_, _) => Err(self.error_nomethod("'*'")),
        }
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
            (Value::Class(lhs), Value::Class(rhs)) => Ok(*lhs == *rhs),
            (Value::Instance(lhs), Value::Instance(rhs)) => Ok(lhs == rhs),
            (Value::Array(lhs), Value::Array(rhs)) => {
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
            (_, _) => return Err(self.error_nomethod("NoMethodError: '>'")),
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
            Value::Range(rref) => {
                let start = self.val_to_s(rref.start);
                let end = self.val_to_s(rref.end);
                let sym = if rref.exclude { "..." } else { ".." };
                format!("({}{}{})", start, sym, end)
            }
            Value::Char(c) => format!("{:x}", c),
            Value::Class(cref) => format! {"Class({})", self.globals.get_ident_name(cref.id)},
            Value::Instance(iref) => {
                format! {"Instance({}:{:?})", self.globals.get_ident_name(iref.classref.id), iref}
            }
            Value::Array(aref) => match aref.elements.len() {
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
        }
    }

    pub fn val_pp(&self, val: PackedValue) -> String {
        match val.unpack() {
            Value::Nil => "nil".to_string(),
            Value::String(s) => format!("\"{}\"", s),
            Value::Class(cref) => format! {"{}", self.globals.get_ident_name(cref.id)},
            Value::Instance(iref) => {
                format! {"#<{}:{:?}>", self.globals.get_ident_name(iref.classref.id), iref}
            }
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
    ) -> VMResult {
        let info = self.globals.get_method_info(methodref);
        #[allow(unused_variables, unused_mut)]
        let mut inst: u8;
        #[cfg(feature = "perf")]
        {
            inst = self.perf.get_prev_inst();
        }
        match info {
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
                Ok(val)
            }
            MethodInfo::AttrReader { id } => match receiver.unpack() {
                Value::Instance(instanceref) => match instanceref.instance_var.get(id) {
                    Some(v) => Ok(v.clone()),
                    None => Ok(PackedValue::nil()),
                },
                _ => unreachable!("AttrReader must be used only for class instance."),
            },
            MethodInfo::AttrWriter { id } => match receiver.unpack() {
                Value::Instance(mut instanceref) => {
                    instanceref.instance_var.insert(*id, args[0]);
                    Ok(args[0])
                }
                _ => unreachable!("AttrReader must be used only for class instance."),
            },
            MethodInfo::RubyFunc {
                params,
                iseq,
                lvars,
                ..
            } => {
                let iseq = iseq.clone();
                self.context_stack
                    .push(Context::new(*lvars, receiver, iseq, methodref));
                let arg_len = args.len();
                for (i, id) in params.clone().iter().enumerate() {
                    *self.lvar_mut(*id) = if i < arg_len {
                        args[i]
                    } else {
                        PackedValue::nil()
                    };
                }

                let res_value = self.vm_run()?;
                #[cfg(feature = "perf")]
                {
                    self.perf.get_perf_no_count(inst);
                }
                self.context_stack.pop().unwrap();
                Ok(res_value)
            }
        }
    }
}

impl VM {
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
        mut class: ClassRef,
        method: IdentId,
    ) -> Result<MethodRef, RubyError> {
        loop {
            match class.get_instance_method(method) {
                Some(methodref) => return Ok(*methodref),
                None => match class.superclass {
                    Some(superclass) => class = superclass,
                    None => {
                        let method_name = self.globals.get_ident_name(method);
                        let class_name = self.globals.get_ident_name(class.id);
                        return Err(self.error_undefined_method(method_name, class_name));
                    }
                },
            };
        }
    }

    pub fn get_toplevel_method(&self, method: IdentId) -> Result<MethodRef, RubyError> {
        match self.globals.get_toplevel_method(method) {
            Some(info) => Ok(info.clone()),
            None => return Err(self.error_unimplemented("Method not defined.")),
        }
    }

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
}
