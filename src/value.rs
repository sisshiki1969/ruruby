#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    FixNum(i64),
}

impl Value {
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::FixNum(_) => true,
            _ => unimplemented!(),
        }
    }
}
