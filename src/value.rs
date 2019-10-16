use crate::class::ClassRef;
use crate::instance::InstanceRef;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    FixNum(i64),
    String(String),
    Class(ClassRef),
    Instance(InstanceRef),
}
