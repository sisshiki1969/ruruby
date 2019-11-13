mod builtin;
mod class;
mod codegen;
mod globals;
mod instance;
mod method;
pub mod value;

use crate::error::{ParseErrKind, RubyError, RuntimeErrKind};
use crate::node::*;
pub use crate::parser::{LvarCollector, LvarId};
pub use crate::util::*;
pub use builtin::*;
pub use class::*;
use codegen::*;
pub use globals::*;
pub use instance::*;
pub use method::*;
use std::collections::HashMap;
pub use value::*;

pub type ValueTable = HashMap<IdentId, PackedValue>;

pub type VMResult = Result<Value, RubyError>;

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
}

#[derive(Debug, Clone)]
pub struct Context {
    pub self_value: PackedValue,
    pub lvar_scope: Vec<PackedValue>,
}

impl Context {
    pub fn new(lvar_num: usize, self_value: PackedValue) -> Self {
        Context {
            self_value,
            lvar_scope: vec![PackedValue::nil(); lvar_num],
        }
    }
}

pub struct Inst;
impl Inst {
    pub const END: u8 = 0;
    pub const PUSH_FIXNUM: u8 = 1;
    pub const PUSH_FLONUM: u8 = 2;
    pub const ADD: u8 = 3;
    pub const SUB: u8 = 4;
    pub const MUL: u8 = 5;
    pub const DIV: u8 = 6;
    pub const EQ: u8 = 7;
    pub const NE: u8 = 8;
    pub const GT: u8 = 9;
    pub const GE: u8 = 10;
    pub const PUSH_TRUE: u8 = 11;
    pub const PUSH_FALSE: u8 = 12;
    pub const PUSH_NIL: u8 = 13;
    pub const SHR: u8 = 14;
    pub const SHL: u8 = 15;
    pub const BIT_OR: u8 = 16;
    pub const BIT_AND: u8 = 17;
    pub const BIT_XOR: u8 = 18;
    pub const JMP: u8 = 19;
    pub const JMP_IF_FALSE: u8 = 20;
    pub const SET_LOCAL: u8 = 21;
    pub const GET_LOCAL: u8 = 22;
    pub const SEND: u8 = 23;
    pub const PUSH_SELF: u8 = 24;
    pub const CREATE_RANGE: u8 = 25;
    pub const GET_CONST: u8 = 26;
    pub const SET_CONST: u8 = 27;
    pub const PUSH_STRING: u8 = 28;
    pub const POP: u8 = 29;
    pub const CONCAT_STRING: u8 = 30;
    pub const TO_S: u8 = 31;
    pub const DEF_CLASS: u8 = 32;
    pub const GET_INSTANCE_VAR: u8 = 33;
    pub const SET_INSTANCE_VAR: u8 = 34;
    pub const DEF_METHOD: u8 = 35;
    pub const DEF_CLASS_METHOD: u8 = 36;
}

impl VM {
    pub fn new(
        ident_table: Option<IdentifierTable>,
        lvar_collector: Option<LvarCollector>,
    ) -> Self {
        let mut globals = Globals::new(ident_table);
        let main_id = globals.get_ident_id(&"main".to_string());
        let main_class = globals.add_class(main_id);
        let vm = VM {
            globals,
            const_table: HashMap::new(),
            codegen: Codegen::new(lvar_collector),
            class_stack: vec![],
            context_stack: vec![Context::new(64, Value::Class(main_class).pack())],
            exec_stack: vec![],
        };
        vm
    }

    pub fn init(&mut self, ident_table: IdentifierTable, lvar_collector: LvarCollector) {
        self.globals.ident_table = ident_table;
        self.codegen.lvar_table = lvar_collector.table;
    }

    /// Get local variable table.
    pub fn lvar_mut(&mut self, id: LvarId) -> &mut PackedValue {
        &mut self.context_stack.last_mut().unwrap().lvar_scope[id.as_usize()]
    }

    pub fn run(&mut self, node: &Node) -> VMResult {
        let iseq = self.codegen.gen_iseq(&mut self.globals, node)?;
        let val = self.vm_run(iseq)?;
        let stack_len = self.exec_stack.len();
        if stack_len != 0 {
            eprintln!("Error: stack length is illegal. {}", stack_len);
        };
        Ok(val.unpack())
    }

    pub fn vm_run(&mut self, iseq: ISeqRef) -> Result<PackedValue, RubyError> {
        let iseq = &*iseq;
        let mut pc = 0;
        loop {
            match iseq[pc] {
                Inst::END => match self.exec_stack.pop() {
                    Some(v) => return Ok(v),
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
                    self.exec_stack.push(Value::String(string).pack());
                    pc += 5;
                }

                Inst::ADD => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_add(lhs, rhs)?;
                    self.exec_stack.push(val);
                    pc += 1;
                }
                Inst::SUB => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_sub(lhs, rhs)?;
                    pc += 1;
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
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let val = self.eval_shr(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val.pack());
                }
                Inst::SHL => {
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let val = self.eval_shl(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val.pack());
                }
                Inst::BIT_AND => {
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let val = self.eval_bitand(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val.pack());
                }
                Inst::BIT_OR => {
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let val = self.eval_bitor(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val.pack());
                }
                Inst::BIT_XOR => {
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let val = self.eval_bitxor(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val.pack());
                }
                Inst::EQ => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_eq(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val);
                }
                Inst::NE => {
                    let lhs = self.exec_stack.pop().unwrap();
                    let rhs = self.exec_stack.pop().unwrap();
                    let val = self.eval_neq(lhs, rhs)?;
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
                            return Err(self.error_unimplemented(format!(
                                "Uninitialized constant '{}'.",
                                name
                            )));
                        }
                    }
                    pc += 5;
                }
                Inst::SET_INSTANCE_VAR => {
                    let var_id = read_id(iseq, pc);
                    let self_var = &self.context_stack.last().unwrap().self_value.unpack();
                    let new_val = self.exec_stack.last().unwrap();
                    match self_var {
                        Value::Instance(id) => self
                            .globals
                            .get_mut_instance_info(*id)
                            .instance_var
                            .insert(var_id, *new_val),
                        Value::Class(id) => self
                            .globals
                            .get_mut_class_info(*id)
                            .instance_var
                            .insert(var_id, *new_val),
                        _ => unreachable!(),
                    };
                    pc += 5;
                }
                Inst::GET_INSTANCE_VAR => {
                    let var_id = read_id(iseq, pc);
                    let self_var = &self.context_stack.last().unwrap().self_value.unpack();
                    let val = match self_var {
                        Value::Instance(id) => {
                            let info = self.globals.get_instance_info(*id);
                            info.instance_var.get(&var_id)
                        }
                        Value::Class(id) => {
                            let info = self.globals.get_class_info(*id);
                            info.instance_var.get(&var_id)
                        }
                        _ => unreachable!(),
                    };
                    let val = match val {
                        Some(val) => val.clone(),
                        None => PackedValue::nil(),
                    };
                    self.exec_stack.push(val);
                    pc += 5;
                }
                Inst::CREATE_RANGE => {
                    let start = self.exec_stack.pop().unwrap();
                    let end = self.exec_stack.pop().unwrap();
                    let exclude = self.exec_stack.pop().unwrap();
                    let range = Value::Range(start, end, self.val_to_bool(exclude));
                    self.exec_stack.push(range.pack());
                    pc += 1;
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
                    //println!("RECV {:?}", receiver);
                    let method_id = read_id(iseq, pc);
                    //println!("METHOD {}", self.ident_table.get_name(method_id));
                    let methodref = match receiver.unpack() {
                        Value::Nil | Value::FixNum(_) => {
                            match self.globals.get_toplevel_method(method_id) {
                                Some(info) => info,
                                None => return Err(self.error_unimplemented("method not defined.")),
                            }
                        }
                        Value::Class(class) => self.get_class_method(class, method_id)?,
                        Value::Instance(instance) => {
                            self.get_instance_method(instance, method_id)?
                        }
                        _ => unimplemented!(),
                    }
                    .clone();
                    let args_num = read32(iseq, pc + 5);
                    let mut args = vec![];
                    for _ in 0..args_num {
                        args.push(self.exec_stack.pop().unwrap());
                    }
                    let val = self.eval_send(&methodref, receiver, args)?;
                    self.exec_stack.push(val);
                    pc += 9;
                }
                Inst::DEF_CLASS => {
                    let id = IdentId::from(read32(iseq, pc + 1));
                    let methodref = MethodRef::from(read32(iseq, pc + 5));
                    let method_info = self.globals.get_method_info(methodref).clone();
                    let classref = self.globals.add_class(id);
                    let val = Value::Class(classref).pack();
                    self.const_table.insert(id, val);

                    let (iseq, lvars) = match &method_info {
                        MethodInfo::RubyFunc { iseq, lvars, .. } => (iseq, lvars),
                        MethodInfo::BuiltinFunc { .. } => unreachable!(),
                    };
                    self.class_stack.push(classref);
                    self.context_stack.push(Context::new(*lvars, val));
                    let _ = self.vm_run(*iseq)?;
                    self.context_stack.pop().unwrap();
                    self.class_stack.pop().unwrap();

                    let id_for_new = self.globals.get_ident_id(&"new".to_string());

                    let new_func_info = MethodInfo::BuiltinFunc {
                        name: "new".to_string(),
                        func: Builtin::builtin_new,
                    };
                    let new_func_ref = self.globals.add_method(new_func_info);
                    let class_info = self.globals.get_mut_class_info(classref);
                    class_info.add_class_method(id_for_new, new_func_ref);
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
                        let classref = self.class_stack.last().unwrap();
                        self.globals
                            .get_mut_class_info(*classref)
                            .add_instance_method(id, methodref);
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
                        let classref = self.class_stack.last().unwrap();
                        self.globals
                            .get_mut_class_info(*classref)
                            .add_class_method(id, methodref);
                    }
                    self.exec_stack.push(PackedValue::nil());
                    pc += 9;
                }
                Inst::TO_S => {
                    let val = self.exec_stack.pop().unwrap();
                    let res = Value::String(self.val_to_s(val)).pack();
                    self.exec_stack.push(res);
                    pc += 1;
                }
                Inst::POP => {
                    self.exec_stack.pop().unwrap();
                    pc += 1;
                }
                _ => unimplemented!("Illegal instruction."),
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
        RubyError::new_runtime_err(RuntimeErrKind::NoMethod(msg.into()), self.codegen.loc)
    }
    pub fn error_unimplemented(&self, msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Unimplemented(msg.into()), self.codegen.loc)
    }
    pub fn error_name(&self, msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Name(msg.into()), self.codegen.loc)
    }
    pub fn error_syntax(&self, msg: impl Into<String>, loc: Loc) -> RubyError {
        RubyError::new_parse_err(ParseErrKind::SyntaxError(msg.into()), loc)
    }
}

impl VM {
    fn eval_add(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<PackedValue, RubyError> {
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
            (_, _) => Err(self.error_nomethod("NoMethodError: '+'")),
        }
    }
    fn eval_sub(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<PackedValue, RubyError> {
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
            (_, _) => Err(self.error_nomethod("NoMethodError: '-'")),
        }
    }

    fn eval_mul(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<PackedValue, RubyError> {
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
            (_, _) => Err(self.error_nomethod("NoMethodError: '*'")),
        }
    }

    fn eval_div(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<PackedValue, RubyError> {
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

    fn eval_shl(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs << rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '<<'")),
        }
    }

    fn eval_shr(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs >> rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    fn eval_bitand(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs & rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    fn eval_bitor(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs | rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    fn eval_bitxor(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs ^ rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    pub fn eval_eq(
        &mut self,
        rhs: PackedValue,
        lhs: PackedValue,
    ) -> Result<PackedValue, RubyError> {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(PackedValue::bool(*lhs == *rhs));
        }
        if lhs.is_packed_num() && rhs.is_packed_num() {
            if lhs.is_packed_fixnum() {
                return Ok(PackedValue::bool(
                    lhs.as_packed_fixnum() as f64 == rhs.as_packed_flonum(),
                ));
            } else if rhs.is_packed_fixnum() {
                return Ok(PackedValue::bool(
                    lhs.as_packed_flonum() == rhs.as_packed_fixnum() as f64,
                ));
            } else {
                return Ok(PackedValue::bool(*lhs == *rhs));
            }
        }
        match (&lhs.unpack(), &rhs.unpack()) {
            (Value::Nil, Value::Nil) => Ok(PackedValue::true_val()),
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::bool(lhs == rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::bool(lhs == rhs)),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(PackedValue::bool(lhs == rhs)),
            (Value::Class(lhs), Value::Class(rhs)) => Ok(PackedValue::bool(lhs == rhs)),
            (Value::Instance(lhs), Value::Instance(rhs)) => Ok(PackedValue::bool(lhs == rhs)),
            _ => Err(self.error_nomethod(format!("NoMethodError: {:?} == {:?}", lhs, rhs))),
        }
    }

    fn eval_neq(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<PackedValue, RubyError> {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(PackedValue::bool(*lhs != *rhs));
        }
        if lhs.is_packed_num() && rhs.is_packed_num() {
            if lhs.is_packed_fixnum() {
                return Ok(PackedValue::bool(
                    lhs.as_packed_fixnum() as f64 != rhs.as_packed_flonum(),
                ));
            } else if rhs.is_packed_fixnum() {
                return Ok(PackedValue::bool(
                    lhs.as_packed_flonum() != rhs.as_packed_fixnum() as f64,
                ));
            } else {
                return Ok(PackedValue::bool(*lhs != *rhs));
            }
        }
        match (lhs.unpack(), rhs.unpack()) {
            (Value::Nil, Value::Nil) => Ok(PackedValue::false_val()),
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::bool(lhs != rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::bool(lhs != rhs)),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(PackedValue::bool(lhs != rhs)),
            (Value::Class(lhs), Value::Class(rhs)) => Ok(PackedValue::bool(lhs != rhs)),
            (Value::Instance(lhs), Value::Instance(rhs)) => Ok(PackedValue::bool(lhs != rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '!='")),
        }
    }

    fn eval_ge(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<PackedValue, RubyError> {
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
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::bool(lhs >= rhs as f64)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::bool(lhs as f64 >= rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::bool(lhs >= rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>='")),
        }
    }

    fn eval_gt(&mut self, rhs: PackedValue, lhs: PackedValue) -> Result<PackedValue, RubyError> {
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
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => PackedValue::bool(lhs > rhs as f64),
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

    pub fn val_to_s(&mut self, val: PackedValue) -> String {
        match val.unpack() {
            Value::Nil => "".to_string(),
            Value::Bool(b) => match b {
                true => "true".to_string(),
                false => "false".to_string(),
            },
            Value::FixNum(i) => i.to_string(),
            Value::FloatNum(f) => f.to_string(),
            Value::String(s) => format!("{}", s),
            //Value::Class(class) => self.get_class_name(*class),
            //Value::Instance(instance) => self.get_instance_name(*instance),
            Value::Range(start, end, exclude) => {
                let start = self.val_to_s(start);
                let end = self.val_to_s(end);
                let sym = if exclude { "..." } else { ".." };
                format!("({}{}{})", start, sym, end)
            }
            Value::Char(c) => format!("{:x}", c),
            Value::Class(id) => format! {"Class({:?})", id},
            Value::Instance(id) => format! {"Instance({:?})", id},
        }
    }
}

impl VM {
    pub fn init_builtin(&mut self) {
        builtin::Builtin::init_builtin(&mut self.globals);
    }

    pub fn eval_send(
        &mut self,
        methodref: &MethodRef,
        receiver: PackedValue,
        args: Vec<PackedValue>,
    ) -> Result<PackedValue, RubyError> {
        let info = self.globals.get_method_info(*methodref);
        match info {
            MethodInfo::BuiltinFunc { func, .. } => {
                let val = func(self, receiver, args)?.pack();
                Ok(val)
            }
            MethodInfo::RubyFunc {
                params,
                iseq,
                lvars,
            } => {
                let iseq = iseq.clone();
                self.context_stack.push(Context::new(*lvars, receiver));
                let arg_len = args.len();
                for (i, id) in params.clone().iter().enumerate() {
                    *self.lvar_mut(*id) = if i < arg_len {
                        args[i]
                    } else {
                        PackedValue::nil()
                    };
                }

                let res_value = self.vm_run(iseq)?;
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
    ) -> Result<&MethodRef, RubyError> {
        match self.globals.get_class_info(class).get_class_method(method) {
            Some(methodref) => Ok(methodref),
            None => match self.globals.get_toplevel_method(method) {
                None => return Err(self.error_nomethod("No class method found.")),
                Some(info) => Ok(info),
            },
        }
    }

    pub fn get_instance_method(
        &self,
        instance: InstanceRef,
        method: IdentId,
    ) -> Result<&MethodRef, RubyError> {
        let classref = self.globals.get_instance_info(instance).get_classref();
        match self
            .globals
            .get_class_info(classref)
            .get_instance_method(method)
        {
            Some(methodref) => Ok(methodref),
            None => match self.globals.get_toplevel_method(method) {
                None => return Err(self.error_nomethod("No instance method found.")),
                Some(info) => Ok(info),
            },
        }
    }
}
