use super::vm_inst::*;
use crate::*;
#[derive(Clone, Default)]
pub struct ISeq(Vec<u8>);

use std::ops::{Index, IndexMut, Range};
use std::{convert::TryInto, fmt};

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

impl Index<ISeqPos> for ISeq {
    type Output = u8;
    fn index(&self, index: ISeqPos) -> &Self::Output {
        &self.0[index.0]
    }
}

impl IndexMut<ISeqPos> for ISeq {
    fn index_mut(&mut self, index: ISeqPos) -> &mut Self::Output {
        &mut self.0[index.0]
    }
}

impl Index<Range<usize>> for ISeq {
    type Output = [u8];
    fn index(&self, range: Range<usize>) -> &Self::Output {
        &self.0[range]
    }
}

impl Index<Range<ISeqPos>> for ISeq {
    type Output = [u8];
    fn index(&self, range: Range<ISeqPos>) -> &Self::Output {
        &self.0[range.start.into_usize()..range.end.into_usize()]
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

    pub fn ident_name(&self, pc: ISeqPos) -> String {
        IdentId::get_name(self.read32(pc).into())
    }

    pub fn push(&mut self, val: u8) {
        self.0.push(val);
    }

    pub fn push32(&mut self, val: u32) {
        self.0.extend_from_slice(&val.to_le_bytes());
    }

    pub fn push64(&mut self, val: u64) {
        self.0.extend_from_slice(&val.to_le_bytes());
    }

    pub fn read8(&self, pc: ISeqPos) -> u8 {
        self[pc]
    }

    pub fn read16(&self, pc: ISeqPos) -> u16 {
        u16::from_le_bytes((&self[pc..pc + 2]).try_into().unwrap())
    }

    pub fn read32(&self, pc: ISeqPos) -> u32 {
        u32::from_le_bytes((&self[pc..pc + 4]).try_into().unwrap())
    }

    pub fn read64(&self, pc: ISeqPos) -> u64 {
        u64::from_le_bytes((&self[pc..pc + 8]).try_into().unwrap())
    }

    pub fn read_block(&self, pc: ISeqPos) -> String {
        match self.read32(pc) {
            0 => "None".to_string(),
            b => format!("MethodId({})", b),
        }
    }

    pub fn write32(&mut self, pc: usize, data: u32) {
        unsafe { std::ptr::write(self[pc] as *mut _, data.to_le_bytes()) };
    }

    pub fn read_usize(&self, pc: ISeqPos) -> usize {
        self.read32(pc) as usize
    }

    pub fn read_id(&self, offset: ISeqPos) -> IdentId {
        self.read32(offset).into()
    }

    pub fn read_lvar_id(&self, offset: ISeqPos) -> LvarId {
        self.read_usize(offset).into()
    }

    pub fn read_method(&self, offset: ISeqPos) -> Option<MethodId> {
        match self.read32(offset) {
            0 => None,
            m => Some(m.into()),
        }
    }

    pub fn read_disp(&self, offset: ISeqPos) -> ISeqDisp {
        ISeqDisp(self.read32(offset) as i32)
    }
}

impl ISeq {
    pub fn push8(&mut self, num: u8) {
        self.push(num as u8);
    }

    pub fn push_argflag(&mut self, flag: ArgFlag) {
        self.push(flag.to_u8());
    }

    pub fn push16(&mut self, num: u16) {
        self.push(num as u8);
        self.push((num >> 8) as u8);
    }

    pub fn push_method(&mut self, block: Option<MethodId>) {
        match block {
            Some(block) => self.push32(block.into()),
            None => self.push32(0),
        };
    }

    /// Write a 32-bit `disp`lacement from `dest` on current ISeqPos.
    pub fn write_disp_from_cur(&mut self, src: ISeqPos) {
        let dest = self.current();
        self.write_disp(src, dest);
    }

    /// Write a 32-bit `disp`lacement of `dest` from `src` on `src` ISeqPos.
    pub fn write_disp(&mut self, src: ISeqPos, dest: ISeqPos) {
        let num = (src - dest).to_i32() as u32;
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

    pub fn gen_pop(&mut self) {
        self.push(Inst::POP);
    }

    pub fn gen_dup(&mut self, len: usize) {
        self.push(Inst::DUP);
        self.push32(len as u32);
    }

    pub fn gen_sinkn(&mut self, len: usize) {
        self.push(Inst::SINKN);
        self.push32(len as u32);
    }

    pub fn gen_topn(&mut self, len: usize) {
        self.push(Inst::TOPN);
        self.push32(len as u32);
    }

    pub fn gen_take(&mut self, len: usize) {
        self.push(Inst::TAKE);
        self.push32(len as u32);
    }

    pub fn gen_concat(&mut self, len: usize) {
        self.push(Inst::CONCAT_STRING);
        self.push32(len as u32);
    }

    pub fn gen_val(&mut self, val: Value) {
        self.push(Inst::PUSH_VAL);
        self.push64(val.id());
    }

    pub fn gen_const_val(&mut self, globals: &mut Globals, val: Value) {
        let id = globals.const_values.insert(val);
        if id > u32::max_value() as usize {
            panic!("Constant value id overflow.")
        };
        self.push(Inst::CONST_VAL);
        self.push32(id as u32);
    }

    pub fn gen_integer(&mut self, globals: &mut Globals, num: i64) {
        let val = Value::integer(num);
        if val.is_packed_value() {
            self.gen_val(val);
        } else {
            self.gen_const_val(globals, val);
        }
    }

    pub fn gen_float(&mut self, globals: &mut Globals, num: f64) {
        let val = Value::float(num);
        if val.is_packed_value() {
            self.gen_val(val);
        } else {
            self.gen_const_val(globals, val);
        }
    }

    pub fn gen_string(&mut self, globals: &mut Globals, s: &str) {
        let val = Value::string(s);
        self.gen_const_val(globals, val);
    }

    pub fn gen_complex(&mut self, globals: &mut Globals, i: Real) {
        let val = Value::complex(Value::integer(0), i.to_val());
        self.gen_const_val(globals, val);
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
        let disp = (self.current() - pos).to_i32() - 5;
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

    pub fn gen_opt_case2(&mut self, map_id: u32) -> ISeqPos {
        self.push(Inst::OPT_CASE2);
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

    pub fn optimize(&mut self) {
        let mut pos = ISeqPos::from(0);
        loop {
            if pos.into_usize() >= self.len() {
                break;
            }
            let inst = self[pos];
            match inst {
                Inst::JMP | Inst::JMP_BACK => {
                    let next_pos = pos + Inst::inst_size(inst);
                    let jmp_dest = next_pos + self.read_disp(next_pos - 4);
                    match self.chase(jmp_dest, true) {
                        DestKind::Dest(dest) => self.write_disp(next_pos, dest),
                        DestKind::Inst(term_inst) => {
                            self[pos] = term_inst;
                            self[pos + 1] = term_inst;
                            self[pos + 2] = term_inst;
                            self[pos + 3] = term_inst;
                            self[pos + 4] = term_inst;
                        }
                    }
                }
                Inst::JMP_T
                | Inst::JMP_F
                | Inst::JMP_F_EQ
                | Inst::JMP_F_NE
                | Inst::JMP_F_GT
                | Inst::JMP_F_GE
                | Inst::JMP_F_LT
                | Inst::JMP_F_LE => {
                    let next_pos = pos + Inst::inst_size(inst);
                    let jmp_dest = next_pos + self.read_disp(next_pos - 4);
                    match self.chase(jmp_dest, false) {
                        DestKind::Dest(dest) => self.write_disp(next_pos, dest),
                        DestKind::Inst(_) => unreachable!(),
                    }
                }
                _ => {}
            }
            pos += Inst::inst_size(inst);
        }
    }

    fn chase(&self, pos: ISeqPos, non_conditional: bool) -> DestKind {
        let inst = self[pos];
        match inst {
            Inst::JMP | Inst::JMP_BACK => {
                let disp = self.read_disp(pos + 1);
                let next_pos = pos + Inst::inst_size(inst) + disp;
                self.chase(next_pos, non_conditional)
            }
            Inst::RETURN | Inst::BREAK | Inst::MRETURN => {
                if non_conditional {
                    DestKind::Inst(inst)
                } else {
                    DestKind::Dest(pos)
                }
            }
            _ => DestKind::Dest(pos),
        }
    }
}
enum DestKind {
    Dest(ISeqPos),
    Inst(u8),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ISeqPos(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ISeqDisp(i32);

impl ISeqDisp {
    pub fn from_i32(disp: i32) -> Self {
        Self(disp)
    }

    pub fn to_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Debug for ISeqPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("ISeqPos({})", self.0))
    }
}

impl std::convert::Into<usize> for ISeqPos {
    fn into(self) -> usize {
        self.0
    }
}

impl std::ops::Add<ISeqDisp> for ISeqPos {
    type Output = Self;
    fn add(self, other: ISeqDisp) -> Self {
        Self(((self.0) as i64 + other.0 as i64) as usize)
    }
}

impl std::ops::AddAssign<ISeqDisp> for ISeqPos {
    fn add_assign(&mut self, other: ISeqDisp) {
        *self = *self + other
    }
}

impl std::ops::Add<usize> for ISeqPos {
    type Output = Self;
    fn add(self, other: usize) -> Self {
        Self(((self.0) as i64 + other as i64) as usize)
    }
}

impl std::ops::AddAssign<usize> for ISeqPos {
    fn add_assign(&mut self, other: usize) {
        *self = *self + other
    }
}

impl std::ops::Sub<usize> for ISeqPos {
    type Output = Self;
    fn sub(self, other: usize) -> Self {
        Self(((self.0) as i64 - other as i64) as usize)
    }
}

impl std::ops::SubAssign<usize> for ISeqPos {
    fn sub_assign(&mut self, other: usize) {
        *self = *self - other
    }
}

impl std::ops::Sub<ISeqPos> for ISeqPos {
    type Output = ISeqDisp;
    fn sub(self, other: ISeqPos) -> Self::Output {
        ISeqDisp((other.0 as i64 - self.0 as i64) as i32)
    }
}

impl ISeqPos {
    pub fn from(pos: usize) -> Self {
        ISeqPos(pos)
    }

    pub fn into_usize(self) -> usize {
        self.0
    }
}
