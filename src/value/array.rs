use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayInfo {
    pub elements: Vec<Value>,
}

impl GC for ArrayInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.elements.iter().for_each(|v| v.mark(alloc));
    }
}

/// Calculate array index.
fn get_array_index(index: i64, len: usize) -> Result<usize, RubyError> {
    if index < 0 {
        let i = len as i64 + index;
        if i < 0 {
            return Err(RubyError::internal("Index too small for array."));
        };
        Ok(i as usize)
    } else {
        Ok(index as usize)
    }
}

impl ArrayInfo {
    pub fn new(elements: Vec<Value>) -> Self {
        ArrayInfo { elements }
    }

    pub fn get_elem(&self, args: &Args) -> VMResult {
        let arg_num = args.len();
        args.check_args_range(1, 2)?;
        let index = args[0].expect_integer("Index")?;
        let self_len = self.elements.len();
        let index = get_array_index(index, self_len).unwrap_or(self_len);
        let val = if arg_num == 1 {
            if index >= self_len {
                Value::nil()
            } else {
                self.elements[index]
            }
        } else {
            let len = args[1].expect_integer("Index")?;
            if len < 0 {
                Value::nil()
            } else if index >= self_len {
                Value::array_from(vec![])
            } else {
                let len = len as usize;
                let end = std::cmp::min(self_len, index + len);
                let ary = (&self.elements[index..end]).to_vec();
                Value::array_from(ary)
            }
        };
        Ok(val)
    }

    pub fn get_elem1(&self, idx: Value) -> VMResult {
        let index = idx.expect_integer("Index")?;
        let self_len = self.elements.len();
        let index = get_array_index(index, self_len).unwrap_or(self_len);
        let val = if index >= self_len {
            Value::nil()
        } else {
            self.elements[index]
        };
        Ok(val)
    }

    pub fn get_elem_imm(&self, index: u32) -> Value {
        if index as usize >= self.elements.len() {
            Value::nil()
        } else {
            self.elements[index as usize]
        }
    }

    pub fn set_elem(&mut self, args: &Args) -> VMResult {
        args.check_args_range(2, 3)?;
        let val = if args.len() == 3 { args[2] } else { args[1] };
        let index = args[0].expect_integer("Index")?;
        let elements = &mut self.elements;
        let len = elements.len();
        if args.len() == 2 {
            if index >= elements.len() as i64 {
                let padding = index as usize - len;
                elements.append(&mut vec![Value::nil(); padding]);
                elements.push(val);
            } else {
                let index = get_array_index(index, len)?;
                elements[index] = val;
            }
        } else {
            let index = get_array_index(index, len)?;
            let length = args[1].expect_integer("Length")?;
            if length < 0 {
                return Err(RubyError::index(format!("Negative length. {}", length)));
            };
            let length = length as usize;
            let end = std::cmp::min(len, index + length);
            match val.as_array() {
                Some(val) => {
                    let mut tail = elements.split_off(end);
                    elements.truncate(index);
                    elements.append(&mut val.elements.clone());
                    elements.append(&mut tail);
                }
                None => {
                    elements.drain(index..end);
                    elements.insert(index, val);
                }
            };
        };
        Ok(val)
    }

    pub fn set_elem1(&mut self, idx: Value, val: Value) -> VMResult {
        let index = idx.expect_integer("Index")?;
        let elements = &mut self.elements;
        let len = elements.len();
        if index >= elements.len() as i64 {
            let padding = index as usize - len;
            elements.extend_from_slice(&vec![Value::nil(); padding]);
            elements.push(val);
        } else {
            let index = get_array_index(index, len)?;
            elements[index] = val;
        }
        Ok(val)
    }

    pub fn set_elem_imm(&mut self, index: u32, val: Value) {
        let elements = &mut self.elements;
        let len = elements.len();
        if index as usize >= len {
            let padding = index as usize - len;
            elements.append(&mut vec![Value::nil(); padding]);
            elements.push(val);
        } else {
            elements[index as usize] = val;
        }
    }

    pub fn to_s(&self, vm: &mut VM) -> Result<String, RubyError> {
        let s = match self.elements.len() {
            0 => "[]".to_string(),
            1 => format!("[{}]", vm.val_inspect(self.elements[0])?),
            len => {
                let mut result = vm.val_inspect(self.elements[0])?;
                for i in 1..len {
                    result = format!("{}, {}", result, vm.val_inspect(self.elements[i])?);
                }
                format! {"[{}]", result}
            }
        };
        Ok(s)
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }
}
