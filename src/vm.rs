use crate::class::*;
use crate::codegen::*;
use crate::error::{ParseErrKind, RubyError, RuntimeErrKind};
use crate::instance::*;
use crate::node::*;
use crate::parser::{LvarCollector, LvarId};
use crate::util::{IdentId, IdentifierTable, Loc};
use crate::value::*;
use std::collections::HashMap;

pub type ValueTable = HashMap<IdentId, Value>;

pub type VMResult = Result<Value, RubyError>;

#[derive(Debug, Clone)]
pub struct VM {
    // Global info
    pub ident_table: IdentifierTable,
    pub class_table: GlobalClassTable,
    pub instance_table: GlobalInstanceTable,
    pub method_table: MethodTable,
    pub const_table: ValueTable,
    pub codegen: Codegen,
    // VM state
    pub context_stack: Vec<Context>,
    pub exec_stack: Vec<PackedValue>,
}

#[derive(Debug, Clone)]
pub struct Context {
    pub lvar_scope: Vec<PackedValue>,
}

impl Context {
    pub fn new(lvar_num: usize) ->Self {
        Context {
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
}

impl VM {
    pub fn new(
        ident_table: Option<IdentifierTable>,
        lvar_collector: Option<LvarCollector>,
    ) -> Self {
        let vm = VM {
            ident_table: match ident_table {
                Some(table) => table,
                None => IdentifierTable::new(),
            },
            class_table: GlobalClassTable::new(),
            instance_table: GlobalInstanceTable::new(),
            method_table: HashMap::new(),
            const_table: HashMap::new(),
            codegen: Codegen::new(lvar_collector),

            context_stack: vec![Context::new(64)],
            exec_stack: vec![],
        };
        vm
    }

    pub fn init(&mut self, ident_table: IdentifierTable, lvar_collector: LvarCollector) {
        self.ident_table = ident_table;
        self.codegen.lvar_table = lvar_collector.table;
    }

    /// Get local variable table.
    pub fn lvar_mut(&mut self, id: LvarId) -> &mut PackedValue {
        &mut self.context_stack.last_mut().unwrap().lvar_scope[id.as_usize()]
    }

    pub fn run(&mut self, node: &Node) -> VMResult {
        //println!("{:?}", node);
        std::mem::swap(&mut self.codegen.ident_table, &mut self.ident_table);
        std::mem::swap(&mut self.codegen.method_table, &mut self.method_table);
        let iseq = self.codegen.gen_iseq(node)?;
        std::mem::swap(&mut self.codegen.ident_table, &mut self.ident_table);
        std::mem::swap(&mut self.codegen.method_table, &mut self.method_table);
        let val = self.vm_run(&iseq)?;
        let stack_len = self.exec_stack.len();
        if stack_len != 0 {
            eprintln!("Error: stack length is illegal. {}", stack_len);
        };
        Ok(val)
    }

    pub fn vm_run(&mut self, iseq: &ISeq) -> VMResult {
        let mut pc = 0;
        loop {
            match iseq[pc] {
                Inst::END => match self.exec_stack.pop() {
                    Some(v) => return Ok(v.unpack()),
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
                    self.exec_stack.push(PackedValue::nil());
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
                    let string = self.ident_table.get_name(id).clone();
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
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let val = self.eval_div(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val.pack());
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
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let val = self.eval_eq(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val.pack());
                }
                Inst::NE => {
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let val = self.eval_neq(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val.pack());
                }
                Inst::GT => {
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let val = self.eval_gt(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val.pack());
                }
                Inst::GE => {
                    let lhs = self.exec_stack.pop().unwrap().unpack();
                    let rhs = self.exec_stack.pop().unwrap().unpack();
                    let val = self.eval_ge(lhs, rhs)?;
                    pc += 1;
                    self.exec_stack.push(val.pack());
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
                    self.const_table.insert(id, val.unpack());
                    pc += 5;
                }
                Inst::GET_CONST => {
                    let id = read_id(iseq, pc);
                    match self.const_table.get(&id) {
                        Some(val) => self.exec_stack.push(val.clone().pack()),
                        None => {
                            let name = self.ident_table.get_name(id).clone();
                            return Err(self.error_unimplemented(format!(
                                "Uninitialized constant '{}'.",
                                name
                            )));
                        }
                    }
                    pc += 5;
                }
                Inst::CREATE_RANGE => {
                    let start = self.exec_stack.pop().unwrap().unpack();
                    let end = self.exec_stack.pop().unwrap().unpack();
                    let exclude = self.exec_stack.pop().unwrap().unpack();
                    let range =
                        Value::Range(Box::new(start), Box::new(end), self.val_to_bool(&exclude));
                    self.exec_stack.push(range.pack());
                    pc += 1;
                }
                Inst::JMP => {
                    let disp = read32(iseq, pc + 1) as i32 as i64;
                    pc = ((pc as i64) + 5 + disp) as usize;
                }
                Inst::JMP_IF_FALSE => {
                    let val = self.exec_stack.pop().unwrap().unpack();
                    if self.val_to_bool(&val) {
                        pc += 5;
                    } else {
                        let disp = read32(iseq, pc + 1) as i32 as i64;
                        pc = ((pc as i64) + 5 + disp) as usize;
                    }
                }
                Inst::SEND => {
                    let receiver = self.exec_stack.pop().unwrap().unpack();
                    //println!("RECV {:?}", receiver);
                    let method_id = read_id(iseq, pc);
                    //println!("METHOD {}", self.ident_table.get_name(method_id));
                    let info = match receiver {
                        Value::Nil | Value::FixNum(_) => match self.method_table.get(&method_id) {
                            Some(info) => info,
                            None => return Err(self.error_unimplemented("method not defined.")),
                        },
                        _ => unimplemented!(),
                    };
                    let args_num = read32(iseq, pc + 5);
                    let mut args = vec![];
                    for _ in 0..args_num {
                        args.push(self.exec_stack.pop().unwrap().unpack());
                    }
                    match info {
                        MethodInfo::BuiltinFunc { func, .. } => {
                            let val = func(self, receiver, args)?.pack();
                            self.exec_stack.push(val);
                        }
                        MethodInfo::RubyFunc {
                            params,
                            iseq,
                            lvars,
                        } => {
                            let func_iseq = iseq.clone();
                            self.context_stack.push(Context::new(*lvars));
                            for (i, id) in params.clone().iter().enumerate() {
                                *self.lvar_mut(*id) = args[i].clone().pack();
                            }

                            let res_value = self.vm_run(&func_iseq)?.pack();
                            self.context_stack.pop().unwrap();
                            self.exec_stack.push(res_value);
                        }
                    }
                    pc += 9;
                }
                Inst::TO_S => {
                    let val = self.exec_stack.pop().unwrap().unpack();
                    let res = Value::String(self.val_to_s(&val)).pack();
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
            IdentId::from_usize(read32(iseq, pc + 1) as usize)
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
                    rhs.as_packed_flonum() - lhs.as_packed_fixnum() as f64,
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
        match (lhs.unpack(), rhs.unpack()) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::fixnum(lhs * rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::flonum(lhs as f64 * rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(PackedValue::flonum(lhs * rhs as f64)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(PackedValue::flonum(lhs * rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '*'")),
        }
    }

    fn eval_div(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs / rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum((lhs as f64) / rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::FloatNum(lhs / (rhs as f64))),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs / rhs)),
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

    pub fn eval_eq(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (&lhs, &rhs) {
            (Value::Nil, Value::Nil) => Ok(Value::Bool(true)),
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::Class(lhs), Value::Class(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::Instance(lhs), Value::Instance(rhs)) => Ok(Value::Bool(lhs == rhs)),
            _ => Err(self.error_nomethod(format!("NoMethodError: {:?} == {:?}", lhs, rhs))),
        }
    }

    fn eval_neq(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::Class(lhs), Value::Class(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::Instance(lhs), Value::Instance(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '!='")),
        }
    }

    fn eval_ge(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs >= rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs >= rhs as f64)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs as f64 >= rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs >= rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>='")),
        }
    }

    fn eval_gt(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs > rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs > rhs as f64)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs as f64 > rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs > rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>'")),
        }
    }
}

impl VM {
    pub fn val_to_bool(&self, val: &Value) -> bool {
        match val {
            Value::Nil | Value::Bool(false) => false,
            _ => true,
        }
    }

    pub fn val_to_s(&mut self, val: &Value) -> String {
        match val {
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
                let sym = if *exclude { "..." } else { ".." };
                format!("({}{}{})", start, sym, end)
            }
            Value::Char(c) => format!("{:x}", c),
            _ => "".to_string(),
        }
    }
}

impl VM {
    pub fn init_builtin(&mut self) {
        crate::builtin::Builtin::init_builtin(&mut self.ident_table, &mut self.method_table);
    }
}
