pub struct Inst;
impl Inst {
    pub const END: u8 = 0;
    pub const PUSH_FIXNUM: u8 = 1;
    pub const PUSH_FLONUM: u8 = 2;
    pub const PUSH_TRUE: u8 = 3;
    pub const PUSH_FALSE: u8 = 4;
    pub const PUSH_NIL: u8 = 5;
    pub const PUSH_STRING: u8 = 6;
    pub const PUSH_SYMBOL: u8 = 7;
    pub const PUSH_SELF: u8 = 8;

    pub const ADD: u8 = 9;
    pub const SUB: u8 = 10;
    pub const MUL: u8 = 11;
    pub const DIV: u8 = 12;
    pub const EQ: u8 = 13;
    pub const NE: u8 = 14;
    pub const GT: u8 = 15;
    pub const GE: u8 = 16;
    pub const SHR: u8 = 17;
    pub const SHL: u8 = 18;
    pub const BIT_OR: u8 = 19;
    pub const BIT_AND: u8 = 20;
    pub const BIT_XOR: u8 = 21;

    pub const SUBI: u8 = 25;
    pub const ADDI: u8 = 26;

    pub const SET_LOCAL: u8 = 30;
    pub const GET_LOCAL: u8 = 31;
    pub const GET_CONST: u8 = 32;
    pub const SET_CONST: u8 = 33;
    pub const GET_INSTANCE_VAR: u8 = 34;
    pub const SET_INSTANCE_VAR: u8 = 35;
    pub const GET_ARRAY_ELEM: u8 = 36;
    pub const SET_ARRAY_ELEM: u8 = 37;

    pub const SEND: u8 = 40;

    pub const CREATE_RANGE: u8 = 50;
    pub const CREATE_ARRAY: u8 = 51;

    pub const POP: u8 = 60;
    pub const CONCAT_STRING: u8 = 61;
    pub const TO_S: u8 = 62;
    pub const DUP: u8 = 63;

    pub const DEF_CLASS: u8 = 70;
    pub const DEF_METHOD: u8 = 71;
    pub const DEF_CLASS_METHOD: u8 = 72;

    pub const JMP: u8 = 80;
    pub const JMP_IF_FALSE: u8 = 81;
}

#[allow(dead_code)]
impl Inst {
    pub fn inst_name(inst: u8) -> &'static str {
        match inst {
            Inst::END => "END",
            Inst::PUSH_FIXNUM => "PUSH_FIXNUM",
            Inst::PUSH_FLONUM => "PUSH_FLONUM",
            Inst::PUSH_TRUE => "PUSH_TRUE",
            Inst::PUSH_FALSE => "PUSH_FALSE",
            Inst::PUSH_NIL => "PUSH_NIL",
            Inst::PUSH_STRING => "PUSH_STRING",
            Inst::PUSH_SYMBOL => "PUSH_SYMBOL",
            Inst::PUSH_SELF => "PUSH_SELF",
            Inst::ADD => "ADD",
            Inst::SUB => "SUB",
            Inst::MUL => "MUL",
            Inst::DIV => "DIV",
            Inst::EQ => "EQ",
            Inst::NE => "NE",
            Inst::GT => "GT",
            Inst::GE => "GE",
            Inst::SHR => "SHR",
            Inst::SHL => "SHL",
            Inst::BIT_OR => "BIT_OR",
            Inst::BIT_AND => "BIT_AND",
            Inst::BIT_XOR => "BIT_XOR",
            Inst::SUBI => "SUBI",
            Inst::ADDI => "ADDI",

            Inst::JMP => "JMP",
            Inst::JMP_IF_FALSE => "JMP_IF_FALSE",
            Inst::SET_LOCAL => "SET_LOCAL",
            Inst::GET_LOCAL => "GET_LOCAL",
            Inst::GET_CONST => "GET_CONST",
            Inst::SET_CONST => "SET_CONST",
            Inst::GET_INSTANCE_VAR => "GET_INST_VAR",
            Inst::SET_INSTANCE_VAR => "SET_INST_VAR",
            Inst::GET_ARRAY_ELEM => "GET_ARY_ELEM",
            Inst::SET_ARRAY_ELEM => "SET_ARY_ELEM",
            Inst::SEND => "SEND",
            Inst::CREATE_RANGE => "CREATE_RANGE",
            Inst::CREATE_ARRAY => "CREATE_ARRAY",
            Inst::POP => "POP",
            Inst::DUP => "DUP",
            Inst::CONCAT_STRING => "CONCAT_STR",
            Inst::TO_S => "TO_S",
            Inst::DEF_CLASS => "DEF_CLASS",
            Inst::DEF_METHOD => "DEF_METHOD",
            Inst::DEF_CLASS_METHOD => "DEF_CLASS_METHOD",
            _ => "undefined",
        }
    }

    pub fn inst_size(inst: u8) -> usize {
        match inst {
            Inst::END
            | Inst::PUSH_NIL
            | Inst::PUSH_TRUE
            | Inst::PUSH_FALSE
            | Inst::PUSH_SELF
            | Inst::ADD
            | Inst::SUB
            | Inst::MUL
            | Inst::DIV
            | Inst::EQ
            | Inst::NE
            | Inst::GT
            | Inst::GE
            | Inst::SHR
            | Inst::SHL
            | Inst::BIT_OR
            | Inst::BIT_AND
            | Inst::BIT_XOR
            | Inst::CONCAT_STRING
            | Inst::CREATE_RANGE
            | Inst::TO_S
            | Inst::POP => 1,

            Inst::PUSH_STRING
            | Inst::PUSH_SYMBOL
            | Inst::SET_LOCAL
            | Inst::GET_LOCAL
            | Inst::GET_CONST
            | Inst::SET_CONST
            | Inst::GET_INSTANCE_VAR
            | Inst::SET_INSTANCE_VAR
            | Inst::GET_ARRAY_ELEM
            | Inst::SET_ARRAY_ELEM
            | Inst::CREATE_ARRAY
            | Inst::JMP
            | Inst::JMP_IF_FALSE
            | Inst::DUP => 5,

            Inst::PUSH_FIXNUM
            | Inst::PUSH_FLONUM
            | Inst::SUBI
            | Inst::ADDI
            | Inst::SEND
            | Inst::DEF_CLASS
            | Inst::DEF_METHOD
            | Inst::DEF_CLASS_METHOD => 9,
            _ => 1,
        }
    }
}
