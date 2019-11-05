use super::class::ClassRef;
use super::instance::InstanceRef;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    FixNum(i64),
    FloatNum(f64),
    String(String),
    Class(ClassRef),
    Instance(InstanceRef),
    Range(Box<Value>, Box<Value>, bool),
    Char(u8),
}
