use crate::*;

pub struct Inst;
impl Inst {
    pub const PUSH_VAL: u8 = 1;
    pub const PUSH_FLONUM: u8 = 2;
    pub const PUSH_NIL: u8 = 5;
    pub const PUSH_SELF: u8 = 8;

    pub const CREATE_RANGE: u8 = 10;
    pub const CREATE_ARRAY: u8 = 11;
    pub const CREATE_PROC: u8 = 12;
    pub const CREATE_HASH: u8 = 13;
    pub const CREATE_REGEXP: u8 = 14;
    pub const CONST_VAL: u8 = 15;

    pub const SET_LOCAL: u8 = 20;
    pub const GET_LOCAL: u8 = 21;
    pub const SET_DYNLOCAL: u8 = 22;
    pub const GET_DYNLOCAL: u8 = 23;
    pub const GET_CONST: u8 = 24;
    pub const SET_CONST: u8 = 25;
    pub const GET_CONST_TOP: u8 = 26;
    pub const GET_SCOPE: u8 = 27;
    pub const GET_IVAR: u8 = 28;
    pub const SET_IVAR: u8 = 29;
    pub const GET_GVAR: u8 = 30;
    pub const SET_GVAR: u8 = 31;
    pub const GET_CVAR: u8 = 32;
    pub const SET_CVAR: u8 = 33;
    pub const GET_SVAR: u8 = 34;
    pub const SET_SVAR: u8 = 35;

    pub const GET_INDEX: u8 = 40;
    pub const SET_INDEX: u8 = 41;
    pub const GET_IDX_I: u8 = 42;
    pub const SET_IDX_I: u8 = 43;

    pub const CHECK_LOCAL: u8 = 50;
    pub const CHECK_CONST: u8 = 51;
    pub const CHECK_SCOPE: u8 = 52;
    pub const CHECK_IVAR: u8 = 53;
    pub const CHECK_GVAR: u8 = 54;
    pub const CHECK_METHOD: u8 = 55;

    pub const SEND: u8 = 60;
    pub const OPT_SEND: u8 = 66;
    pub const OPT_SEND_N: u8 = 68;

    pub const POP: u8 = 80;
    pub const DUP: u8 = 81;
    pub const TAKE: u8 = 82;
    pub const SPLAT: u8 = 83;
    pub const CONCAT_STRING: u8 = 84;
    pub const TO_S: u8 = 85;
    pub const SINKN: u8 = 86;
    pub const TOPN: u8 = 87;

    pub const DEF_CLASS: u8 = 90;
    pub const DEF_SCLASS: u8 = 91;
    pub const DEF_METHOD: u8 = 92;
    pub const DEF_SMETHOD: u8 = 93;

    pub const JMP: u8 = 100;
    pub const JMP_BACK: u8 = 101;
    pub const JMP_F: u8 = 102;
    pub const JMP_T: u8 = 103;
    pub const RETURN: u8 = 104;
    pub const BREAK: u8 = 105;
    pub const OPT_CASE: u8 = 106;
    pub const MRETURN: u8 = 107;
    pub const YIELD: u8 = 108;
    pub const RESCUE: u8 = 109;
    pub const THROW: u8 = 110;
    pub const OPT_CASE2: u8 = 111;
    pub const SUPER: u8 = 112;
    
    pub const ADD: u8 = 120;
    pub const SUB: u8 = 121;
    pub const MUL: u8 = 122;
    pub const DIV: u8 = 123;
    pub const REM: u8 = 124;
    pub const EQ: u8 = 125;
    pub const NE: u8 = 126;
    pub const TEQ: u8 = 127;
    pub const GT: u8 = 128;
    pub const GE: u8 = 129;
    pub const LT: u8 = 130;
    pub const LE: u8 = 131;
    pub const NOT: u8 = 132;
    pub const SHR: u8 = 133;
    pub const SHL: u8 = 134;
    pub const BOR: u8 = 135;
    pub const BAND: u8 = 136;
    pub const BXOR: u8 = 137;
    pub const BNOT: u8 = 138;
    pub const POW: u8 = 139;
    pub const CMP: u8 = 140;
    pub const NEG: u8 = 141;

    pub const ADDI: u8 = 150;
    pub const SUBI: u8 = 151;
    pub const EQI: u8 = 152;
    pub const NEI: u8 = 153;
    pub const GTI: u8 = 154;
    pub const GEI: u8 = 155;
    pub const LTI: u8 = 156;
    pub const LEI: u8 = 157;

    pub const JMP_F_EQ: u8 = 170;
    pub const JMP_F_NE: u8 = 171;
    pub const JMP_F_GT: u8 = 172;
    pub const JMP_F_GE: u8 = 173;
    pub const JMP_F_LT: u8 = 174;
    pub const JMP_F_LE: u8 = 175;

    pub const JMP_F_EQI: u8 = 180;
    pub const JMP_F_NEI: u8 = 181;
    pub const JMP_F_GTI: u8 = 182;
    pub const JMP_F_GEI: u8 = 183;
    pub const JMP_F_LTI: u8 = 184;
    pub const JMP_F_LEI: u8 = 185;
}

impl Inst {
    pub fn inst_name(inst: u8) -> String {
        let inst = match inst {
            Inst::PUSH_VAL => "PUSH_VAL",
            Inst::PUSH_NIL => "PUSH_NIL",
            Inst::PUSH_SELF => "PUSH_SELF",

            Inst::ADD => "ADD",
            Inst::SUB => "SUB",
            Inst::MUL => "MUL",
            Inst::DIV => "DIV",
            Inst::REM => "REM",
            Inst::EQ => "EQ",
            Inst::NE => "NE",
            Inst::TEQ => "TEQ",
            Inst::GT => "GT",
            Inst::GE => "GE",
            Inst::LT => "LT",
            Inst::LE => "LE",
            Inst::NOT => "NOT",
            Inst::SHR => "SHR",
            Inst::SHL => "SHL",
            Inst::BOR => "BIT_OR",
            Inst::BAND => "BIT_AND",
            Inst::BXOR => "BIT_XOR",
            Inst::BNOT => "BIT_NOT",
            Inst::POW => "POW",
            Inst::CMP => "CMP",
            Inst::NEG => "NEG",

            Inst::ADDI => "ADDI",
            Inst::SUBI => "SUBI",
            Inst::EQI => "EQI",
            Inst::NEI => "NEI",
            Inst::GTI => "GTI",
            Inst::GEI => "GEI",
            Inst::LTI => "LTI",
            Inst::LEI => "LEI",

            Inst::JMP_F_EQ => "JMP_F_EQ",
            Inst::JMP_F_NE => "JMP_F_NE",
            Inst::JMP_F_GT => "JMP_F_GT",
            Inst::JMP_F_GE => "JMP_F_GE",
            Inst::JMP_F_LT => "JMP_F_LT",
            Inst::JMP_F_LE => "JMP_F_LE",

            Inst::JMP_F_EQI => "JMP_F_EQI",
            Inst::JMP_F_NEI => "JMP_F_NEI",
            Inst::JMP_F_GTI => "JMP_F_GTI",
            Inst::JMP_F_GEI => "JMP_F_GEI",
            Inst::JMP_F_LTI => "JMP_F_LTI",
            Inst::JMP_F_LEI => "JMP_F_LEI",

            Inst::SET_LOCAL => "SET_LOCAL",
            Inst::GET_LOCAL => "GET_LOCAL",
            Inst::SET_DYNLOCAL => "SET_DYNLOCAL",
            Inst::GET_DYNLOCAL => "GET_DYNLOCAL",
            Inst::GET_CONST => "GET_CONST",
            Inst::SET_CONST => "SET_CONST",
            Inst::GET_CONST_TOP => "GET_CONSTTOP",
            Inst::GET_SCOPE => "GET_SCOPE",

            Inst::GET_IVAR => "GET_IVAR",
            Inst::SET_IVAR => "SET_IVAR",
            Inst::GET_CVAR => "GET_CVAR",
            Inst::SET_CVAR => "SET_CVAR",
            Inst::GET_GVAR => "GET_GVAR",
            Inst::SET_GVAR => "SET_GVAR",
            Inst::GET_SVAR => "GET_SVAR",
            Inst::SET_SVAR => "SET_SVAR",
            Inst::GET_INDEX => "GET_INDEX",
            Inst::SET_INDEX => "SET_INDEX",
            Inst::GET_IDX_I => "GET_IDX_I",
            Inst::SET_IDX_I => "SET_IDX_I",

            Inst::CHECK_LOCAL => "CHECK_LOCAL",
            Inst::CHECK_CONST => "CHECK_CONST",
            Inst::CHECK_SCOPE => "CHECK_SCOPE",
            Inst::CHECK_IVAR => "CHECK_IVAR",
            Inst::CHECK_GVAR => "CHECK_GVAR",
            Inst::CHECK_METHOD => "CHECK_METHOD",

            Inst::SEND => "SEND",
            Inst::OPT_SEND => "O_SEND",
            Inst::OPT_SEND_N => "O_SEND_N",

            Inst::CREATE_RANGE => "CREATE_RANGE",
            Inst::CREATE_ARRAY => "CREATE_ARRAY",
            Inst::CREATE_PROC => "CREATE_PROC",
            Inst::CREATE_HASH => "CREATE_HASH",
            Inst::CREATE_REGEXP => "CREATE_REGEX",
            Inst::CONST_VAL => "CONST_VAL",

            Inst::POP => "POP",
            Inst::DUP => "DUP",
            Inst::TAKE => "TAKE",
            Inst::SPLAT => "SPLAT",
            Inst::CONCAT_STRING => "CONCAT_STR",
            Inst::TO_S => "TO_S",
            Inst::SINKN => "SINKN",
            Inst::TOPN => "TOPN",

            Inst::DEF_CLASS => "DEF_CLASS",
            Inst::DEF_SCLASS => "DEF_SCLASS",
            Inst::DEF_METHOD => "DEF_METHOD",
            Inst::DEF_SMETHOD => "DEF_CMETHOD",

            Inst::JMP => "JMP",
            Inst::JMP_BACK => "JMP_BACK",
            Inst::JMP_F => "JMP_IF_F",
            Inst::JMP_T => "JMP_IF_T",
            Inst::RETURN => "RETURN",
            Inst::BREAK => "BREAK",
            Inst::OPT_CASE => "OPT_CASE",
            Inst::OPT_CASE2 => "OPT_CASE2",
            Inst::MRETURN => "MRETURN",
            Inst::YIELD => "YIELD",
            Inst::RESCUE => "RESCUE",
            Inst::THROW => "THROW",
            Inst::SUPER => "SUPER",

            _ => return format!("undefined {}", inst),
        };
        inst.to_string()
    }

    pub fn inst_size(inst: u8) -> ISeqDisp {
        let disp = match inst {
            Inst::RETURN
            | Inst::PUSH_NIL
            | Inst::PUSH_SELF
            | Inst::ADD                 
            | Inst::SUB                 
            | Inst::MUL                 
            | Inst::DIV                 
            | Inst::REM
            | Inst::POW
            | Inst::TEQ
            | Inst::EQ
            | Inst::NE
            | Inst::GT
            | Inst::GE
            | Inst::LT
            | Inst::LE
            | Inst::CMP
            | Inst::NOT
            | Inst::SHL
            | Inst::SHR
            | Inst::NEG
            | Inst::BOR
            | Inst::BAND
            | Inst::BXOR
            | Inst::BNOT
            | Inst::CREATE_RANGE
            | Inst::CREATE_REGEXP
            | Inst::GET_INDEX
            | Inst::SET_INDEX
            | Inst::TO_S
            | Inst::SPLAT
            | Inst::POP
            | Inst::BREAK
            | Inst::MRETURN
            | Inst::THROW => 1,
                                        // operand
            Inst::SET_LOCAL           // LvarId: u32
            | Inst::GET_LOCAL           // LVarId: u32
            | Inst::SET_CONST           // IdentId: u32
            | Inst::CHECK_CONST         // IdentId: u32
            | Inst::CHECK_SCOPE         // IdentId: u32
            | Inst::CHECK_METHOD        // IdentId: u32
            | Inst::GET_CONST_TOP       // IdentId: u32
            | Inst::GET_SCOPE           // IdentId: u32
            | Inst::GET_IVAR            // IdentId: u32
            | Inst::SET_IVAR            // IdentId: u32
            | Inst::CHECK_IVAR          // IdentId: u32
            | Inst::GET_GVAR            // IdentId: u32
            | Inst::SET_GVAR            // IdentId: u32
            | Inst::CHECK_GVAR          // IdentId: u32
            | Inst::GET_SVAR            // id: u32
            | Inst::SET_SVAR            // id: u32
            | Inst::GET_CVAR            // IdentId: u32
            | Inst::SET_CVAR            // IdentId: u32
            | Inst::GET_IDX_I           // immediate: u32
            | Inst::SET_IDX_I           // immediate: u32
            | Inst::CREATE_ARRAY        // number of items: u32
            | Inst::CONST_VAL           // ConstId: u32

            | Inst::JMP                 // disp: i32
            | Inst::JMP_BACK            // disp: i32
            | Inst::JMP_F               // disp: i32
            | Inst::JMP_T               // disp: i32

            | Inst::JMP_F_EQ            // disp: i32
            | Inst::JMP_F_NE            // disp: i32
            | Inst::JMP_F_GT            // disp: i32
            | Inst::JMP_F_GE            // disp: i32
            | Inst::JMP_F_LT            // disp: i32
            | Inst::JMP_F_LE            // disp: i32

            | Inst::DUP                 // number of items: u32
            | Inst::TAKE                // number of items: u32
            | Inst::CONCAT_STRING       // number of items: u32
            | Inst::SINKN               // number of items: u32
            | Inst::TOPN                // number of items: u32
            | Inst::ADDI                // immediate: i32
            | Inst::SUBI                // immediate: i32
            | Inst::EQI                 // immediate: i32
            | Inst::NEI                 // immediate: i32
            | Inst::GTI                 // immediate: i32
            | Inst::GEI                 // immediate: i32
            | Inst::LTI                 // immediate: i32
            | Inst::LEI                 // immediate: i32
            | Inst::CREATE_PROC         // block: u32
            | Inst::DEF_SCLASS          // block: u32
            | Inst::CREATE_HASH         // number of items: u32
            | Inst::YIELD               // number of items: u32
            | Inst::RESCUE              // number of items: u32
            => 5,

            Inst::SUPER  => 8,
            // number of args: u16 / block: u32 / flag: u8

            Inst::PUSH_VAL              // value: Value
            | Inst::SET_DYNLOCAL
            | Inst::GET_DYNLOCAL
            | Inst::CHECK_LOCAL
            | Inst::GET_CONST           // IdentId: u32 / cache: u32
            | Inst::OPT_CASE
            | Inst::OPT_CASE2
            | Inst::DEF_METHOD          // method_id: u32 / method: u32
            | Inst::DEF_SMETHOD         // method_id: u32 / method: u32

            | Inst::JMP_F_EQI           // immediate: i32 / disp: i32
            | Inst::JMP_F_NEI           // immediate: i32 / disp: i32
            | Inst::JMP_F_GTI           // immediate: i32 / disp: i32
            | Inst::JMP_F_GEI           // immediate: i32 / disp: i32
            | Inst::JMP_F_LTI           // immediate: i32 / disp: i32
            | Inst::JMP_F_LEI           // immediate: i32 / disp: i32
            => 9,
            Inst::DEF_CLASS => 10,      // is_module: u8 / method_id: u32 / block: u32
            Inst::OPT_SEND | Inst::OPT_SEND_N   => 15,
                    // method_id: u32 / number of args: u16 / block: u32 / icache: u32
            Inst::SEND  => 16,
                    // method_id: u32 / number of args: u16 / flag: u8 / block: u32 / icache: u32
            _ => panic!("unimplemented instruction."),
        };
        ISeqDisp::from_i32(disp)
    }
}
