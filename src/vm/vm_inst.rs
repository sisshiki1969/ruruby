#[cfg(feature = "perf")]
use crate::vm::PerfCounter;

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

    pub const JMP: u8 = 25;
    pub const JMP_IF_FALSE: u8 = 26;

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

    pub const DEF_CLASS: u8 = 70;
    pub const DEF_METHOD: u8 = 71;
    pub const DEF_CLASS_METHOD: u8 = 72;
}

impl Inst {
    #[allow(dead_code)]
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
            Inst::CONCAT_STRING => "CONCAT_STR",
            Inst::TO_S => "TO_S",
            Inst::DEF_CLASS => "DEF_CLASS",
            Inst::DEF_METHOD => "DEF_METHOD",
            Inst::DEF_CLASS_METHOD => "DEF_CLASS_METHOD",
            _ => "undefined",
        }
    }
}

#[cfg(feature = "perf")]
impl Inst {
    pub fn print_perf(counter: &Vec<PerfCounter>) {
        eprintln!("Performance analysis for Inst:");
        eprintln!("------------------------------------------");
        eprintln!(
            "{:<12} {:>10} {:>8} {:>8}",
            "Inst name", "count", "%time", "nsec"
        );
        eprintln!("{:<12} {:>10} {:>8} {:>8}", "", "", "", "/inst");
        eprintln!("------------------------------------------");
        let mut sum = std::time::Duration::from_secs(0);
        for c in counter {
            sum += c.duration;
        }
        for (
            i,
            PerfCounter {
                count: c,
                duration: d,
            },
        ) in counter.iter().enumerate()
        {
            if *c == 0 {
                continue;
            }
            eprintln!(
                "{:<12} {:>10} {:>8.2} {:>8}",
                Inst::inst_name(i as u8),
                if *c > 10000_000 {
                    format!("{:>9}M", c / 1000_000)
                } else if *c > 10000 {
                    format!("{:>9}K", c / 1000)
                } else {
                    format!("{:>10}", *c)
                },
                (d.as_micros() as f64) * 100.0 / (sum.as_micros() as f64),
                d.as_nanos() / (*c as u128)
            );
        }
    }
}
