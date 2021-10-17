use crate::Value;
use std::ops::{Index, IndexMut, Range};
use std::pin::Pin;

pub(super) const VM_STACK_SIZE: usize = 8192;

#[derive(Clone)]
pub(super) struct RubyStack {
    len: usize,
    buf: Pin<Box<[Value]>>,
}

impl std::fmt::Debug for RubyStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &self.buf[0..self.len])
    }
}

impl Index<usize> for RubyStack {
    type Output = Value;
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len);
        &self.buf[index]
    }
}

impl IndexMut<usize> for RubyStack {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.len);
        &mut self.buf[index]
    }
}

impl Index<Range<usize>> for RubyStack {
    type Output = [Value];
    fn index(&self, index: std::ops::Range<usize>) -> &Self::Output {
        assert!(index.end <= self.len);
        &self.buf[index]
    }
}

impl IndexMut<Range<usize>> for RubyStack {
    fn index_mut(&mut self, index: std::ops::Range<usize>) -> &mut Self::Output {
        assert!(index.end <= self.len);
        &mut self.buf[index]
    }
}

impl RubyStack {
    pub(super) fn new() -> Self {
        Self {
            len: 0,
            buf: Pin::from(unsafe { Box::new_uninit_slice(VM_STACK_SIZE).assume_init() }),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.len
    }

    pub(super) fn truncate(&mut self, len: usize) {
        if len >= self.len {
            return;
        }
        self.len = len;
    }

    pub(super) fn resize(&mut self, new_len: usize, value: Value) {
        if new_len > VM_STACK_SIZE {
            panic!("Stack overflow")
        }
        let len = self.len();

        if new_len > len {
            self.buf[len..new_len].fill(value);
            self.len = new_len;
        } else {
            self.truncate(new_len);
        }
    }

    pub(super) fn copy_within(&mut self, src: std::ops::Range<usize>, dest: usize) {
        self.buf.copy_within(src, dest);
    }

    pub(super) fn remove(&mut self, index: usize) -> Value {
        let v = self.buf[index];
        let len = self.len();
        self.buf.copy_within(index + 1..len, index);
        self.len -= 1;
        v
    }

    pub(super) fn insert(&mut self, index: usize, element: Value) {
        let len = self.len();
        self.buf.copy_within(index..len, index + 1);
        self.buf[index] = element;
        self.len += 1;
    }

    pub(super) fn push(&mut self, val: Value) {
        if self.len == VM_STACK_SIZE {
            panic!("Stack overflow.");
        }
        self.buf[self.len] = val;
        self.len += 1;
    }

    pub(super) fn pop(&mut self) -> Option<Value> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(self.buf[self.len])
        }
    }

    pub(super) fn last(&self) -> Option<Value> {
        if self.len == 0 {
            None
        } else {
            Some(self.buf[self.len - 1])
        }
    }

    pub(super) fn iter(&self) -> std::slice::Iter<Value> {
        let len = self.len;
        self.buf[0..len].iter()
    }

    pub(super) fn extend_from_slice(&mut self, src: &[Value]) {
        let len = src.len();
        self.buf[self.len..self.len + len].copy_from_slice(src);
        self.len += len;
    }

    pub(super) fn extend_from_within(&mut self, src: std::ops::Range<usize>) {
        let len = src.len();
        self.copy_within(src, self.len);
        self.len += len;
    }

    pub(super) fn split_off(&mut self, at: usize) -> Vec<Value> {
        let len = self.len;
        self.len = at;
        self.buf[at..len].to_vec()
    }

    pub(super) fn drain(&mut self, range: std::ops::Range<usize>) -> std::slice::Iter<Value> {
        self.len -= range.len();
        self.buf[range].iter()
    }

    pub(super) fn as_mut_ptr(&mut self) -> *mut Value {
        self.buf.as_mut_ptr()
    }

    pub(super) fn as_ptr(&self) -> *const Value {
        self.buf.as_ptr()
    }
}
