use super::StackPtr;
use crate::Value;
use std::ops::{Index, IndexMut, Range};
use std::pin::Pin;

pub(super) const VM_STACK_SIZE: usize = 8192;

///
/// Ruruby exection stack.
///
/// Ruruby stack is implemented as a fixed and pinned heap array of `Value`s.
/// You can do operations like push, pop, or extend, as if it was a Vec.
///
/// Stack size is VM_STACK_SIZE(currently, 8192 `Value`s).
#[derive(Clone)]
pub(super) struct RubyStack {
    /// Stack pointer.
    pub sp: StackPtr,
    /// Pinned Buffer.
    buf: Pin<Box<[Value]>>,
}

impl std::fmt::Debug for RubyStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", &self.buf[0..self.len()])
    }
}

impl Index<usize> for RubyStack {
    type Output = Value;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        debug_assert!(index < self.len());
        &self.buf[index]
    }
}

impl IndexMut<usize> for RubyStack {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        debug_assert!(index < self.len());
        &mut self.buf[index]
    }
}

impl Index<Range<usize>> for RubyStack {
    type Output = [Value];
    #[inline(always)]
    fn index(&self, index: std::ops::Range<usize>) -> &Self::Output {
        debug_assert!(index.end <= self.len());
        &self.buf[index]
    }
}

impl IndexMut<Range<usize>> for RubyStack {
    #[inline(always)]
    fn index_mut(&mut self, index: std::ops::Range<usize>) -> &mut Self::Output {
        debug_assert!(index.end <= self.len());
        &mut self.buf[index]
    }
}

impl RubyStack {
    /// Allocate new `RubyStack`.
    pub(super) fn new() -> Self {
        let mut inner = unsafe { Box::new_uninit_slice(VM_STACK_SIZE).assume_init() };
        let sp = StackPtr::from(inner.as_mut_ptr());
        Self {
            sp,
            buf: Pin::from(inner),
        }
    }

    pub(super) fn check_boundary(&self, p: *mut Value) -> bool {
        let ptr = self.buf.as_ptr() as *mut Value;
        ptr <= p && p < unsafe { ptr.add(VM_STACK_SIZE) }
    }

    /// Set SP to `new_len`.
    #[inline(always)]
    unsafe fn set_len(&mut self, new_len: usize) {
        self.sp = StackPtr::from(self.buf.as_mut_ptr()) + new_len;
    }

    /// Increment SP.
    #[inline(always)]
    unsafe fn inc_len(&mut self, offset: usize) {
        self.sp = self.sp + offset;
    }

    /// Decrement SP.
    #[inline(always)]
    unsafe fn dec_len(&mut self, offset: usize) {
        self.sp = self.sp - offset;
    }

    /// Get length of stack.
    /// This is as same as the index of SP in the stack.
    #[inline(always)]
    pub(super) fn len(&self) -> usize {
        let len = unsafe { self.sp.as_ptr().offset_from(self.buf.as_ptr()) };
        assert!(len >= 0);
        len as usize
    }

    /// Shortens the stack.
    /// If len is greater than the current length, this has no effect.
    #[inline(always)]
    pub(super) fn truncate(&mut self, len: usize) {
        if len >= self.len() {
            return;
        }
        unsafe { self.set_len(len) };
    }

    /// Resize the stack so that len is equal to new_len.
    /// If new_len is greater than len, the stack is extended by the difference, with each additional slot filled with Nil.
    /// If new_len is less than len, the stack is simply truncated.
    pub(super) fn resize(&mut self, new_len: usize) {
        assert!(new_len <= VM_STACK_SIZE);
        let len = self.len();
        if new_len > len {
            self.buf[len..new_len].fill(Value::nil());
        }
        unsafe { self.set_len(new_len) };
    }

    pub(super) fn grow(&mut self, offset: usize) {
        debug_assert!(self.len() + offset <= VM_STACK_SIZE);
        unsafe {
            std::slice::from_raw_parts_mut(self.sp.as_ptr(), offset).fill(Value::nil());
            self.inc_len(offset)
        };
    }

    pub(super) fn copy_within(&mut self, src: std::ops::Range<usize>, dest: usize) {
        self.buf.copy_within(src, dest);
    }

    pub(super) fn remove(&mut self, index: usize) -> Value {
        let v = self.buf[index];
        let len = self.len();
        self.buf.copy_within(index + 1..len, index);
        unsafe { self.dec_len(1) };
        v
    }

    pub(super) fn insert(&mut self, index: usize, element: Value) {
        let len = self.len();
        self.buf.copy_within(index..len, index + 1);
        self.buf[index] = element;
        unsafe { self.inc_len(1) };
    }

    #[inline(always)]
    pub(super) fn push(&mut self, val: Value) {
        debug_assert!(self.len() != VM_STACK_SIZE);
        unsafe {
            *(self.sp.as_ptr()) = val;
            self.inc_len(1)
        };
    }

    #[inline(always)]
    pub(super) fn pop(&mut self) -> Value {
        debug_assert!(self.len() != 0);
        unsafe {
            self.dec_len(1);
            *self.sp.as_ptr()
        }
    }

    #[inline(always)]
    pub(super) fn pop2(&mut self) -> (Value, Value) {
        debug_assert!(self.len() >= 2);
        unsafe {
            self.dec_len(2);
            let ptr = self.sp.as_ptr();
            (*ptr, *(ptr.add(1)))
        }
    }

    #[inline(always)]
    pub(super) fn last(&self) -> Value {
        debug_assert!(self.len() != 0);
        unsafe { *self.sp.as_ptr().sub(1) }
    }

    pub(super) fn iter(&self) -> std::slice::Iter<Value> {
        let len = self.len();
        self.buf[0..len].iter()
    }

    pub(super) fn extend_from_slice(&mut self, src: &[Value]) {
        let src_len = src.len();
        unsafe {
            std::slice::from_raw_parts_mut(self.sp.as_ptr(), src_len).copy_from_slice(src);
            self.inc_len(src_len)
        };
    }

    pub(super) fn extend_from_within(&mut self, src: std::ops::Range<usize>) {
        let len = src.len();
        self.copy_within(src, self.len());
        unsafe { self.inc_len(len) };
    }

    pub(super) fn split_off(&mut self, at: usize) -> Vec<Value> {
        let len = self.len();
        unsafe { self.set_len(at) };
        self.buf[at..len].to_vec()
    }

    pub(super) fn drain(&mut self, range: std::ops::Range<usize>) -> std::slice::Iter<Value> {
        unsafe { self.dec_len(range.len()) };
        self.buf[range].iter()
    }

    #[inline(always)]
    pub(super) fn as_ptr(&self) -> *const Value {
        self.buf.as_ptr()
    }
}

#[cfg(test)]
mod test {
    use super::RubyStack;
    use super::Value;

    #[test]
    fn stack1() {
        let mut stack = RubyStack::new();
        assert_eq!(0, stack.len());
        stack.push(Value::fixnum(5));
        assert_eq!(1, stack.len());
        stack.push(Value::fixnum(7));
        assert_eq!(2, stack.len());
        stack.push(Value::fixnum(42));
        assert_eq!(3, stack.len());
        assert_eq!(Value::fixnum(42), stack.last());
        let v = stack.pop();
        assert_eq!(42, v.as_fixnum().unwrap());
        assert_eq!(2, stack.len());
        let v = stack.pop();
        assert_eq!(7, v.as_fixnum().unwrap());
        assert_eq!(1, stack.len());
        let v = stack.pop();
        assert_eq!(5, v.as_fixnum().unwrap());
        assert_eq!(0, stack.len());
    }

    #[test]
    fn stack2() {
        let mut stack = RubyStack::new();
        stack.push(Value::fixnum(5));
        stack.push(Value::fixnum(7));
        stack.push(Value::fixnum(42));
        stack.push(Value::fixnum(97));
        assert_eq!(4, stack.len());
        stack.truncate(2);
        assert_eq!(2, stack.len());
        assert_eq!(5, stack[0].as_fixnum().unwrap());
        assert_eq!(7, stack[1].as_fixnum().unwrap());
        stack.resize(4);
        stack.truncate(4);
        assert_eq!(5, stack[0].as_fixnum().unwrap());
        assert_eq!(7, stack[1].as_fixnum().unwrap());
        assert_eq!(Value::nil(), stack[2]);
        assert_eq!(Value::nil(), stack[3]);
        assert_eq!(4, stack.len());
        stack[3] = Value::fixnum(99);
        assert_eq!(4, stack.len());
        assert_eq!(5, stack[0].as_fixnum().unwrap());
        assert_eq!(7, stack[1].as_fixnum().unwrap());
        assert_eq!(Value::nil(), stack[2]);
        assert_eq!(99, stack[3].as_fixnum().unwrap());
        stack.extend_from_slice(&[Value::fixnum(34), Value::fixnum(56)]);
        assert_eq!(6, stack.len());
        assert_eq!(5, stack[0].as_fixnum().unwrap());
        assert_eq!(7, stack[1].as_fixnum().unwrap());
        assert_eq!(Value::nil(), stack[2]);
        assert_eq!(99, stack[3].as_fixnum().unwrap());
        assert_eq!(34, stack[4].as_fixnum().unwrap());
        assert_eq!(56, stack[5].as_fixnum().unwrap());
        stack.copy_within(3..6, 2);
        assert_eq!(6, stack.len());
        assert_eq!(5, stack[0].as_fixnum().unwrap());
        assert_eq!(7, stack[1].as_fixnum().unwrap());
        assert_eq!(99, stack[2].as_fixnum().unwrap());
        assert_eq!(34, stack[3].as_fixnum().unwrap());
        assert_eq!(56, stack[4].as_fixnum().unwrap());
        assert_eq!(56, stack[5].as_fixnum().unwrap());
        stack.remove(4);
        assert_eq!(5, stack.len());
        assert_eq!(5, stack[0].as_fixnum().unwrap());
        assert_eq!(7, stack[1].as_fixnum().unwrap());
        assert_eq!(99, stack[2].as_fixnum().unwrap());
        assert_eq!(34, stack[3].as_fixnum().unwrap());
        assert_eq!(56, stack[4].as_fixnum().unwrap());
        stack.remove(1);
        assert_eq!(4, stack.len());
        assert_eq!(5, stack[0].as_fixnum().unwrap());
        assert_eq!(99, stack[1].as_fixnum().unwrap());
        assert_eq!(34, stack[2].as_fixnum().unwrap());
        assert_eq!(56, stack[3].as_fixnum().unwrap());
        stack.insert(1, Value::fixnum(42));
        assert_eq!(5, stack.len());
        assert_eq!(5, stack[0].as_fixnum().unwrap());
        assert_eq!(42, stack[1].as_fixnum().unwrap());
        assert_eq!(99, stack[2].as_fixnum().unwrap());
        assert_eq!(34, stack[3].as_fixnum().unwrap());
        assert_eq!(56, stack[4].as_fixnum().unwrap());
        assert_eq!(
            vec![Value::fixnum(34), Value::fixnum(56)],
            stack.split_off(3)
        );
        assert_eq!(3, stack.len());
        assert_eq!(5, stack[0].as_fixnum().unwrap());
        assert_eq!(42, stack[1].as_fixnum().unwrap());
        assert_eq!(99, stack[2].as_fixnum().unwrap());
    }

    #[test]
    fn stack3() {
        let mut stack = RubyStack::new();
        stack.push(Value::fixnum(3));
        stack.grow(2);
        assert_eq!(3, stack.len());
        assert_eq!(3, stack[0].as_fixnum().unwrap());
        assert_eq!(Value::nil(), stack[1]);
        assert_eq!(Value::nil(), stack[2]);
    }
}
