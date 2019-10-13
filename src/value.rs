#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    FixNum(i64),
    String(String),
}

impl Value {
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::FixNum(_) => true,
            Value::String(s) => true,
            _ => unimplemented!(),
        }
    }

    pub fn to_s(&self) -> String {
        match self {
            Value::Nil => "".to_string(),
            Value::Bool(b) => match b {
                true => "true".to_string(),
                false => "false".to_string(),
            },
            Value::FixNum(i) => i.to_string(),
            Value::String(s) => s.clone(),
        }
    }
}
