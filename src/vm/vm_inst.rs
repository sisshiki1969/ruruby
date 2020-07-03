use crate::*;

pub struct Inst;
impl Inst {
    pub const PUSH_FIXNUM: u8 = 1;
    pub const PUSH_FLONUM: u8 = 2;
    pub const PUSH_TRUE: u8 = 3;
    pub const PUSH_FALSE: u8 = 4;
    pub const PUSH_NIL: u8 = 5;
    pub const PUSH_SYMBOL: u8 = 7;
    pub const PUSH_SELF: u8 = 8;

    pub const ADD: u8 = 10;
    pub const SUB: u8 = 11;
    pub const MUL: u8 = 12;
    pub const DIV: u8 = 13;
    pub const REM: u8 = 14;
    pub const EQ: u8 = 15;
    pub const NE: u8 = 16;
    pub const TEQ: u8 = 17;
    pub const GT: u8 = 18;
    pub const GE: u8 = 19;
    pub const NOT: u8 = 20;
    pub const SHR: u8 = 21;
    pub const SHL: u8 = 22;
    pub const BIT_OR: u8 = 23;
    pub const BIT_AND: u8 = 24;
    pub const BIT_XOR: u8 = 25;
    pub const BIT_NOT: u8 = 26;
    pub const POW: u8 = 27;
    pub const CMP: u8 = 28;

    pub const ADDI: u8 = 30;
    pub const SUBI: u8 = 31;
    pub const IVAR_ADDI: u8 = 32;
    pub const B_ANDI: u8 = 33;
    pub const B_ORI: u8 = 34;
    pub const EQI: u8 = 35;
    pub const NEI: u8 = 36;
    pub const LVAR_ADDI: u8 = 37;

    pub const SET_LOCAL: u8 = 40;
    pub const GET_LOCAL: u8 = 41;
    pub const SET_DYNLOCAL: u8 = 42;
    pub const GET_DYNLOCAL: u8 = 43;
    pub const GET_CONST: u8 = 44;
    pub const SET_CONST: u8 = 45;
    pub const GET_CONST_TOP: u8 = 46;
    pub const GET_SCOPE: u8 = 47;

    pub const GET_IVAR: u8 = 48;
    pub const SET_IVAR: u8 = 49;
    pub const GET_GVAR: u8 = 50;
    pub const SET_GVAR: u8 = 51;
    pub const GET_INDEX: u8 = 52;
    pub const SET_INDEX: u8 = 53;

    pub const CHECK_LOCAL: u8 = 54;

    pub const SEND: u8 = 60;
    pub const SEND_SELF: u8 = 61;
    pub const OPT_SEND: u8 = 62;
    pub const OPT_SEND_SELF: u8 = 63;

    pub const CREATE_RANGE: u8 = 70;
    pub const CREATE_ARRAY: u8 = 71;
    pub const CREATE_PROC: u8 = 72;
    pub const CREATE_HASH: u8 = 73;
    pub const CREATE_REGEXP: u8 = 74;
    pub const CONST_VAL: u8 = 75;

    pub const POP: u8 = 80;
    pub const DUP: u8 = 81;
    pub const TAKE: u8 = 82;
    pub const SPLAT: u8 = 83;
    pub const CONCAT_STRING: u8 = 84;
    pub const TO_S: u8 = 85;

    pub const DEF_CLASS: u8 = 90;
    pub const DEF_METHOD: u8 = 91;
    pub const DEF_SMETHOD: u8 = 92;

    pub const JMP: u8 = 100;
    pub const JMP_IF_FALSE: u8 = 101;
    pub const END: u8 = 102;
    pub const RETURN: u8 = 103;
    pub const OPT_CASE: u8 = 104;
    pub const MRETURN: u8 = 105;
    pub const YIELD: u8 = 106;
}

#[allow(dead_code)]
impl Inst {
    pub fn inst_name(inst: u8) -> String {
        match inst {
            Inst::PUSH_FIXNUM => "PUSH_FIXNUM".to_string(),
            Inst::PUSH_FLONUM => "PUSH_FLONUM".to_string(),
            Inst::PUSH_TRUE => "PUSH_TRUE".to_string(),
            Inst::PUSH_FALSE => "PUSH_FALSE".to_string(),
            Inst::PUSH_NIL => "PUSH_NIL".to_string(),
            Inst::PUSH_SYMBOL => "PUSH_SYMBOL".to_string(),
            Inst::PUSH_SELF => "PUSH_SELF".to_string(),

            Inst::ADD => "ADD".to_string(),
            Inst::SUB => "SUB".to_string(),
            Inst::MUL => "MUL".to_string(),
            Inst::DIV => "DIV".to_string(),
            Inst::REM => "REM".to_string(),
            Inst::EQ => "EQ".to_string(),
            Inst::NE => "NE".to_string(),
            Inst::TEQ => "TEQ".to_string(),
            Inst::GT => "GT".to_string(),
            Inst::GE => "GE".to_string(),
            Inst::NOT => "NOT".to_string(),
            Inst::SHR => "SHR".to_string(),
            Inst::SHL => "SHL".to_string(),
            Inst::BIT_OR => "BIT_OR".to_string(),
            Inst::BIT_AND => "BIT_AND".to_string(),
            Inst::BIT_XOR => "BIT_XOR".to_string(),
            Inst::BIT_NOT => "BIT_NOT".to_string(),
            Inst::POW => "POW".to_string(),
            Inst::CMP => "CMP".to_string(),

            Inst::ADDI => "ADDI".to_string(),
            Inst::SUBI => "SUBI".to_string(),
            Inst::IVAR_ADDI => "IVAR_ADDI".to_string(),
            Inst::B_ANDI => "B_ANDI".to_string(),
            Inst::B_ORI => "B_ORI".to_string(),
            Inst::EQI => "EQI".to_string(),
            Inst::NEI => "NEI".to_string(),
            Inst::LVAR_ADDI => "LVAR_ADDI".to_string(),

            Inst::SET_LOCAL => "SET_LOCAL".to_string(),
            Inst::GET_LOCAL => "GET_LOCAL".to_string(),
            Inst::SET_DYNLOCAL => "SET_DYNLOCAL".to_string(),
            Inst::GET_DYNLOCAL => "GET_DYNLOCAL".to_string(),
            Inst::GET_CONST => "GET_CONST".to_string(),
            Inst::SET_CONST => "SET_CONST".to_string(),
            Inst::GET_CONST_TOP => "GET_CONSTTOP".to_string(),
            Inst::GET_SCOPE => "GET_SCOPE".to_string(),

            Inst::GET_IVAR => "GET_IVAR".to_string(),
            Inst::SET_IVAR => "SET_IVAR".to_string(),
            Inst::GET_GVAR => "GET_GVAR".to_string(),
            Inst::SET_GVAR => "SET_GVAR".to_string(),
            Inst::GET_INDEX => "GET_INDEX".to_string(),
            Inst::SET_INDEX => "SET_INDEX".to_string(),

            Inst::CHECK_LOCAL => "CHECK_LOCAL".to_string(),

            Inst::SEND => "SEND".to_string(),
            Inst::SEND_SELF => "SENDSLF".to_string(),
            Inst::OPT_SEND => "OPT_SEND".to_string(),
            Inst::OPT_SEND_SELF => "OPT_SENDSLF".to_string(),

            Inst::CREATE_RANGE => "CREATE_RANGE".to_string(),
            Inst::CREATE_ARRAY => "CREATE_ARRAY".to_string(),
            Inst::CREATE_PROC => "CREATE_PROC".to_string(),
            Inst::CREATE_HASH => "CREATE_HASH".to_string(),
            Inst::CREATE_REGEXP => "CREATE_REGEX".to_string(),
            Inst::CONST_VAL => "CONST_VAL".to_string(),

            Inst::POP => "POP".to_string(),
            Inst::DUP => "DUP".to_string(),
            Inst::TAKE => "TAKE".to_string(),
            Inst::SPLAT => "SPLAT".to_string(),
            Inst::CONCAT_STRING => "CONCAT_STR".to_string(),
            Inst::TO_S => "TO_S".to_string(),

            Inst::DEF_CLASS => "DEF_CLASS".to_string(),
            Inst::DEF_METHOD => "DEF_METHOD".to_string(),
            Inst::DEF_SMETHOD => "DEF_CMETHOD".to_string(),

            Inst::JMP => "JMP".to_string(),
            Inst::JMP_IF_FALSE => "JMP_IF_FALSE".to_string(),
            Inst::END => "END".to_string(),
            Inst::RETURN => "RETURN".to_string(),
            Inst::OPT_CASE => "OPT_CASE".to_string(),
            Inst::MRETURN => "MRETURN".to_string(),
            Inst::YIELD => "YIELD".to_string(),

            _ => format!("undefined {}", inst),
        }
    }

    pub fn inst_size(inst: u8) -> usize {
        match inst {
            Inst::END
            | Inst::PUSH_NIL
            | Inst::PUSH_TRUE
            | Inst::PUSH_FALSE
            | Inst::PUSH_SELF
            | Inst::REM
            | Inst::POW
            | Inst::TEQ
            | Inst::EQ
            | Inst::NE
            | Inst::GT
            | Inst::GE
            | Inst::CMP
            | Inst::NOT
            | Inst::SHR
            | Inst::BIT_OR
            | Inst::BIT_AND
            | Inst::BIT_XOR
            | Inst::BIT_NOT
            | Inst::CREATE_RANGE
            | Inst::CREATE_REGEXP
            | Inst::TO_S
            | Inst::SPLAT
            | Inst::POP
            | Inst::RETURN
            | Inst::MRETURN => 1,
                                        // operand
            Inst::PUSH_SYMBOL         // IdentId: u32
            | Inst::SET_LOCAL           // LvarId: u32
            | Inst::GET_LOCAL           // LVarId: u32
            | Inst::GET_CONST           // IdentId: u32
            | Inst::SET_CONST           // IdentId: u32
            | Inst::GET_CONST_TOP       // IdentId: u32
            | Inst::GET_SCOPE           // IdentId: u32
            | Inst::GET_IVAR            // IdentId: u32
            | Inst::SET_IVAR            // IdentId: u32
            | Inst::GET_GVAR            // IdentId: u32
            | Inst::SET_GVAR            // IdentId: u32
            | Inst::GET_INDEX
            | Inst::SET_INDEX
            | Inst::CREATE_ARRAY        // number of items: u32
            | Inst::CREATE_PROC
            | Inst::CONST_VAL           // ConstId: u32
            | Inst::JMP                 // disp: u32
            | Inst::JMP_IF_FALSE        // disp: u32
            | Inst::DUP                 // number of items: u32
            | Inst::TAKE                // number of items: u32
            | Inst::CONCAT_STRING       // number of items: u32
            | Inst::ADD                 // inline cache: u32
            | Inst::SUB                 // inline cache: u32
            | Inst::MUL                 // inline cache: u32
            | Inst::DIV                 // inline cache: u32
            | Inst::ADDI                // immediate: i32
            | Inst::SUBI                // immediate: i32
            | Inst::B_ANDI              // immediate: i32
            | Inst::B_ORI               // immediate: i32
            | Inst::EQI                 // immediate: i32
            | Inst::NEI                 // immediate: i32
            | Inst::SHL                 // inline cache: u32
            | Inst::CREATE_HASH         // number of items: u32
            | Inst::YIELD               // number of items: u32
            => 5,

            Inst::PUSH_FIXNUM
            | Inst::PUSH_FLONUM
            | Inst::SET_DYNLOCAL
            | Inst::GET_DYNLOCAL
            | Inst::DEF_METHOD
            | Inst::DEF_SMETHOD
            | Inst::OPT_CASE
            | Inst::CHECK_LOCAL
            | Inst::IVAR_ADDI
            | Inst::LVAR_ADDI => 9,
            Inst::DEF_CLASS => 10,
            Inst::OPT_SEND | Inst::OPT_SEND_SELF => 11,
            Inst::SEND | Inst::SEND_SELF => 17,
            _ => panic!(),
        }
    }

    pub fn inst_info(globals: &Globals, iseq_ref: ISeqRef, pc: usize) -> String {
        fn imm_i32(iseq: &Vec<u8>, pc: usize) -> String {
            format!(
                "{} {}",
                Inst::inst_name(iseq[pc]),
                Inst::read32(iseq, pc + 1) as i32
            )
        }
        let iseq = &iseq_ref.iseq;
        let id_lock = ID.read().unwrap();
        match iseq[pc] {
            Inst::ADDI | Inst::SUBI | Inst::B_ANDI | Inst::B_ORI | Inst::EQI | Inst::NEI => {
                imm_i32(iseq, pc)
            }
            Inst::IVAR_ADDI => format!(
                "IVAR_ADDI {} +{}",
                Inst::ident_name(iseq, pc + 1),
                Inst::read32(iseq, pc + 5) as i32
            ),
            Inst::LVAR_ADDI => {
                let id = Inst::read32(iseq, pc + 1) as usize;
                let ident_id = iseq_ref.lvar.get_name(LvarId::from_usize(id));
                let name = id_lock.get_ident_name(ident_id);
                format!(
                    "LVAR_ADDI '{}' LvarId:{} +{}",
                    name,
                    id,
                    Inst::read32(iseq, pc + 5) as i32
                )
            }
            Inst::PUSH_FIXNUM => format!("PUSH_FIXNUM {}", Inst::read64(iseq, pc + 1) as i64),
            Inst::PUSH_FLONUM => {
                format!("PUSH_FLONUM {}", f64::from_bits(Inst::read64(iseq, pc + 1)))
            }

            Inst::JMP | Inst::JMP_IF_FALSE => format!(
                "{} {:>05x}",
                Inst::inst_name(iseq[pc]),
                pc as i32 + 5 + Inst::read32(iseq, pc + 1) as i32
            ),
            Inst::OPT_CASE => format!(
                "OPT_CASE {:>05}",
                pc as i32 + 13 + Inst::read32(iseq, pc + 9) as i32,
            ),
            Inst::SET_LOCAL | Inst::GET_LOCAL => {
                let id = Inst::read32(iseq, pc + 1);
                let ident_id = iseq_ref.lvar.get_name(LvarId::from_u32(id));
                let name = id_lock.get_ident_name(ident_id);
                format!("{} '{}' LvarId:{}", Inst::inst_name(iseq[pc]), name, id)
            }
            Inst::SET_DYNLOCAL | Inst::GET_DYNLOCAL => {
                let frame = Inst::read32(iseq, pc + 5);
                let id = Inst::read32(iseq, pc + 1);
                //let ident_id = iseq_ref.lvar.get_name(LvarId::from_u32(id));
                //let name = id_lock.get_ident_name(ident_id);
                format!(
                    "{} outer:{} LvarId:{}",
                    Inst::inst_name(iseq[pc]),
                    frame,
                    id
                )
            }
            Inst::CHECK_LOCAL => {
                let frame = Inst::read32(iseq, pc + 5);
                let id = Inst::read32(iseq, pc + 1) as usize;
                let ident_id = iseq_ref.lvar.get_name(LvarId::from_usize(id));
                let name = id_lock.get_ident_name(ident_id);
                format!("CHECK_LOCAL '{}' outer:{} LvarId:{}", name, frame, id)
            }
            Inst::PUSH_SYMBOL
            | Inst::GET_CONST
            | Inst::GET_CONST_TOP
            | Inst::SET_CONST
            | Inst::GET_SCOPE
            | Inst::GET_IVAR
            | Inst::SET_IVAR => format!(
                "{} '{}'",
                Inst::inst_name(iseq[pc]),
                Inst::ident_name(iseq, pc + 1)
            ),

            Inst::GET_INDEX => format!("GET_INDEX {} items", Inst::read32(iseq, pc + 1)),
            Inst::SET_INDEX => format!("SET_INDEX {} items", Inst::read32(iseq, pc + 1)),
            Inst::SEND | Inst::SEND_SELF => format!(
                "{} '{}' {} items",
                Inst::inst_name(iseq[pc]),
                Inst::ident_name(iseq, pc + 1),
                Inst::read16(iseq, pc + 5)
            ),
            Inst::OPT_SEND | Inst::OPT_SEND_SELF => format!(
                "{} '{}' {} items",
                Inst::inst_name(iseq[pc]),
                Inst::ident_name(iseq, pc + 1),
                Inst::read16(iseq, pc + 5)
            ),
            Inst::CREATE_ARRAY
            | Inst::CREATE_PROC
            | Inst::CREATE_HASH
            | Inst::DUP
            | Inst::TAKE
            | Inst::CONCAT_STRING => format!(
                "{} {} items",
                Inst::inst_name(iseq[pc]),
                Inst::read32(iseq, pc + 1)
            ),
            Inst::CONST_VAL => {
                let id = Inst::read32(iseq, pc + 1);
                format!("CONST_VAL {:?}", globals.const_values.get(id as usize))
            }
            Inst::DEF_CLASS => format!(
                "DEF_CLASS {} '{}' method:{}",
                if Inst::read8(iseq, pc + 1) == 1 {
                    "module"
                } else {
                    "class"
                },
                Inst::ident_name(iseq, pc + 2),
                Inst::read32(iseq, pc + 6)
            ),
            Inst::DEF_METHOD => format!("DEF_METHOD '{}'", Inst::ident_name(iseq, pc + 1)),
            Inst::DEF_SMETHOD => format!("DEF_SMETHOD '{}'", Inst::ident_name(iseq, pc + 1)),
            _ => format!("{}", Inst::inst_name(iseq[pc])),
        }
    }

    fn read64(iseq: &ISeq, pc: usize) -> u64 {
        let ptr = iseq[pc..pc + 1].as_ptr() as *const u64;
        unsafe { *ptr }
    }

    fn read32(iseq: &ISeq, pc: usize) -> u32 {
        let ptr = iseq[pc..pc + 1].as_ptr() as *const u32;
        unsafe { *ptr }
    }

    fn read16(iseq: &ISeq, pc: usize) -> u16 {
        let ptr = iseq[pc..pc + 1].as_ptr() as *const u16;
        unsafe { *ptr }
    }

    fn read8(iseq: &ISeq, pc: usize) -> u8 {
        iseq[pc]
    }

    fn ident_name(iseq: &ISeq, pc: usize) -> String {
        IdentId::get_name(IdentId::from(Inst::read32(iseq, pc))).to_string()
    }
}
