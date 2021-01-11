use super::vm_inst::*;
use crate::*;
#[derive(Clone, Default)]
pub struct ISeq(Vec<u8>);

use std::fmt;
use std::ops::{Index, IndexMut, Range};
impl Index<usize> for ISeq {
    type Output = u8;

    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl IndexMut<usize> for ISeq {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

impl Index<Range<usize>> for ISeq {
    type Output = [u8];

    fn index(&self, range: Range<usize>) -> &Self::Output {
        &self.0[range]
    }
}

impl fmt::Debug for ISeq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl ISeq {
    pub fn new() -> Self {
        ISeq(vec![])
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn current(&self) -> ISeqPos {
        ISeqPos::from(self.0.len())
    }

    pub fn ident_name(&self, pc: usize) -> String {
        IdentId::get_name(self.read32(pc).into())
    }

    pub fn push(&mut self, val: u8) {
        self.0.push(val);
    }

    pub fn read8(&self, pc: usize) -> u8 {
        self[pc]
    }

    pub fn read16(&self, pc: usize) -> u16 {
        let ptr = self[pc..pc + 1].as_ptr() as *const u16;
        unsafe { *ptr }
    }

    pub fn read32(&self, pc: usize) -> u32 {
        let ptr = self[pc..pc + 1].as_ptr() as *const u32;
        unsafe { *ptr }
    }

    pub fn write32(&self, pc: usize, data: u32) {
        let ptr = self[pc..pc + 1].as_ptr() as *mut u32;
        unsafe { *ptr = data }
    }

    pub fn read64(&self, pc: usize) -> u64 {
        let ptr = self[pc..pc + 1].as_ptr() as *const u64;
        unsafe { *ptr }
    }

    pub fn read_usize(&self, pc: usize) -> usize {
        self.read32(pc) as usize
    }

    pub fn read_id(&self, offset: usize) -> IdentId {
        self.read32(offset).into()
    }

    pub fn read_lvar_id(&self, offset: usize) -> LvarId {
        self.read_usize(offset).into()
    }

    pub fn read_methodref(&self, offset: usize) -> MethodRef {
        self.read64(offset).into()
    }

    pub fn read_disp(&self, offset: usize) -> i64 {
        self.read32(offset) as i32 as i64
    }
}

impl ISeq {
    pub fn push8(&mut self, num: u8) {
        self.push(num as u8);
    }

    pub fn push16(&mut self, num: u16) {
        self.push(num as u8);
        self.push((num >> 8) as u8);
    }

    pub fn push32(&mut self, num: u32) {
        self.push(num as u8);
        self.push((num >> 8) as u8);
        self.push((num >> 16) as u8);
        self.push((num >> 24) as u8);
    }

    pub fn push64(&mut self, num: u64) {
        self.push(num as u8);
        self.push((num >> 8) as u8);
        self.push((num >> 16) as u8);
        self.push((num >> 24) as u8);
        self.push((num >> 32) as u8);
        self.push((num >> 40) as u8);
        self.push((num >> 48) as u8);
        self.push((num >> 56) as u8);
    }

    /// Write a 32-bit `disp`lacement from `dest` on current ISeqPos.
    pub fn write_disp_from_cur(&mut self, src: ISeqPos) {
        let dest = self.current();
        self.write_disp(src, dest);
    }

    /// Write a 32-bit `disp`lacement of `dest` from `src` on `src` ISeqPos.
    pub fn write_disp(&mut self, src: ISeqPos, dest: ISeqPos) {
        let num = src.disp(dest) as u32;
        self[src.0 - 4] = (num >> 0) as u8;
        self[src.0 - 3] = (num >> 8) as u8;
        self[src.0 - 2] = (num >> 16) as u8;
        self[src.0 - 1] = (num >> 24) as u8;
    }
}

impl ISeq {
    pub fn gen_push_nil(&mut self) {
        self.push(Inst::PUSH_NIL);
    }

    pub fn gen_push_self(&mut self) {
        self.push(Inst::PUSH_SELF);
    }

    pub fn gen_fixnum(&mut self, num: i64) {
        self.push(Inst::PUSH_FIXNUM);
        self.push64(num as u64);
    }

    pub fn gen_const_val(&mut self, id: usize) {
        if id > u32::max_value() as usize {
            panic!("Constant value id overflow.")
        };
        self.push(Inst::CONST_VAL);
        self.push32(id as u32);
    }

    pub fn gen_string(&mut self, globals: &mut Globals, s: &str) {
        let val = Value::string(s);
        let id = globals.const_values.insert(val);
        self.gen_const_val(id);
    }

    pub fn gen_complex(&mut self, globals: &mut Globals, i: Real) {
        let val = Value::complex(Value::integer(0), i.to_val());
        let id = globals.const_values.insert(val);
        self.gen_const_val(id);
    }

    pub fn gen_symbol(&mut self, id: IdentId) {
        self.push(Inst::PUSH_SYMBOL);
        self.push32(id.into());
    }

    pub fn gen_subi(&mut self, i: i32) {
        self.push(Inst::SUBI);
        self.push32(i as u32);
    }

    pub fn gen_create_array(&mut self, len: usize) {
        self.push(Inst::CREATE_ARRAY);
        self.push32(len as u32);
    }

    pub fn gen_create_hash(&mut self, len: usize) {
        self.push(Inst::CREATE_HASH);
        self.push32(len as u32);
    }

    pub fn gen_create_regexp(&mut self) {
        self.push(Inst::CREATE_REGEXP);
    }

    pub fn gen_set_array_elem(&mut self) {
        self.push(Inst::SET_INDEX);
    }

    pub fn gen_splat(&mut self) {
        self.push(Inst::SPLAT);
    }

    pub fn gen_jmp_if_f(&mut self) -> ISeqPos {
        self.push(Inst::JMP_F);
        self.push32(0);
        self.current()
    }

    pub fn gen_jmp_if_t(&mut self) -> ISeqPos {
        self.push(Inst::JMP_T);
        self.push32(0);
        self.current()
    }

    pub fn gen_jmp_back(&mut self, pos: ISeqPos) {
        let disp = self.current().disp(pos) - 5;
        self.push(Inst::JMP_BACK);
        self.push32(disp as u32);
    }

    pub fn gen_jmp(&mut self) -> ISeqPos {
        self.push(Inst::JMP);
        self.push32(0);
        self.current()
    }

    pub fn gen_return(&mut self) {
        self.push(Inst::RETURN);
    }

    pub fn gen_break(&mut self) {
        self.push(Inst::BREAK);
    }

    pub fn gen_method_return(&mut self) {
        self.push(Inst::MRETURN);
    }

    pub fn gen_opt_case(&mut self, map_id: u32) -> ISeqPos {
        self.push(Inst::OPT_CASE);
        self.push32(map_id);
        self.push32(0);
        self.current()
    }

    pub fn gen_get_instance_var(&mut self, id: IdentId) {
        self.push(Inst::GET_IVAR);
        self.push32(id.into());
    }

    pub fn gen_set_instance_var(&mut self, id: IdentId) {
        self.push(Inst::SET_IVAR);
        self.push32(id.into());
    }

    pub fn gen_ivar_addi(&mut self, id: IdentId, val: u32, use_value: bool) {
        self.push(Inst::IVAR_ADDI);
        self.push32(id.into());
        self.push32(val);
        if use_value {
            self.gen_get_instance_var(id);
        }
    }

    pub fn gen_get_global_var(&mut self, id: IdentId) {
        self.push(Inst::GET_GVAR);
        self.push32(id.into());
    }

    pub fn gen_set_global_var(&mut self, id: IdentId) {
        self.push(Inst::SET_GVAR);
        self.push32(id.into());
    }

    pub fn gen_set_const(&mut self, id: IdentId) {
        self.push(Inst::SET_CONST);
        self.push32(id.into());
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ISeqPos(usize);

impl fmt::Debug for ISeqPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("ISeqPos({})", self.0))
    }
}

impl ISeqPos {
    pub fn from(pos: usize) -> Self {
        ISeqPos(pos)
    }

    pub fn to_usize(&self) -> usize {
        self.0
    }

    pub fn disp(&self, dist: ISeqPos) -> i32 {
        let dist = dist.0 as i64;
        (dist - (self.0 as i64)) as i32
    }
}
