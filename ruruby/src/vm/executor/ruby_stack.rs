use crate::{ControlFrame, Frame, Value, CF};
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

impl Index<Frame> for RubyStack {
    type Output = Value;
    #[inline(always)]
    fn index(&self, index: Frame) -> &Self::Output {
        &self.buf[index.0]
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
    pub(super) fn resize_to(&mut self, new_sp: StackPtr) {
        debug_assert!(new_sp <= self.sp + VM_STACK_SIZE);
        if new_sp > self.sp {
            unsafe {
                std::slice::from_raw_parts_mut(self.sp.0, (new_sp - self.sp) as usize)
                    .fill(Value::nil());
            }
        }
        self.sp = new_sp;
    }

    #[inline(always)]
    pub(super) fn grow(&mut self, offset: usize) {
        debug_assert!(self.len() + offset <= VM_STACK_SIZE);
        unsafe { std::slice::from_raw_parts_mut(self.sp.0, offset).fill(Value::nil()) }
        self.sp += offset;
    }

    pub(super) fn copy_within(&mut self, src: std::ops::Range<usize>, dest: usize) {
        self.buf.copy_within(src, dest);
    }

    pub(super) fn remove(&mut self, index: usize) -> Value {
        let v = self.buf[index];
        let len = self.len();
        self.buf.copy_within(index + 1..len, index);
        self.sp -= 1;
        v
    }

    pub(super) fn remove_ptr(&mut self, index: StackPtr) -> Value {
        let v = index[0];
        //let len = self.len();
        //self.buf.copy_within(index + 1..len, index);
        RubyStack::stack_copy_within(index, 1..(self.sp - index) as usize, 0);
        self.sp -= 1;
        v
    }

    pub(super) fn insert(&mut self, index: usize, element: Value) {
        let len = self.len();
        self.buf.copy_within(index..len, index + 1);
        self.buf[index] = element;
        self.sp += 1;
    }

    #[inline(always)]
    pub(super) fn push(&mut self, val: Value) {
        debug_assert!(self.len() != VM_STACK_SIZE);
        self.sp[0] = val;
        self.sp += 1;
    }

    #[inline(always)]
    pub(super) fn pop(&mut self) -> Value {
        debug_assert!(self.len() != 0);
        self.sp -= 1;
        self.sp[0]
    }

    #[inline(always)]
    pub(super) fn pop2(&mut self) -> (Value, Value) {
        debug_assert!(self.len() >= 2);
        self.sp -= 2;
        let ptr = self.sp;
        (ptr[0], ptr[1])
    }

    #[inline(always)]
    pub(super) fn last(&self) -> Value {
        debug_assert!(self.len() != 0);
        self.sp[-1]
    }

    pub(super) fn iter(&self) -> std::slice::Iter<Value> {
        let len = self.len();
        self.buf[0..len].iter()
    }

    pub(super) fn extend_from_slice(&mut self, src: &[Value]) {
        let src_len = src.len();
        unsafe {
            std::slice::from_raw_parts_mut(self.sp.as_ptr(), src_len).copy_from_slice(src);
        };
        self.sp += src_len;
    }

    pub(super) fn extend_from_within(&mut self, src: std::ops::Range<usize>) {
        let len = src.len();
        self.copy_within(src, self.len());
        self.sp += len;
    }

    pub(super) fn stack_copy_within(mut ptr: StackPtr, src: std::ops::Range<usize>, dest: usize) {
        ptr[0..usize::max(src.start, dest) + (src.end - src.start)].copy_within(src, dest);
    }

    #[inline(always)]
    pub(super) fn as_ptr(&self) -> *const Value {
        self.buf.as_ptr()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, std::cmp::PartialOrd)]
pub struct StackPtr(*mut Value);

impl std::default::Default for StackPtr {
    #[inline(always)]
    fn default() -> Self {
        StackPtr(std::ptr::null_mut())
    }
}

impl std::ops::Add<usize> for StackPtr {
    type Output = Self;
    #[inline(always)]
    fn add(self, other: usize) -> Self {
        Self(unsafe { self.0.add(other) })
    }
}

impl std::ops::AddAssign<usize> for StackPtr {
    #[inline(always)]
    fn add_assign(&mut self, other: usize) {
        unsafe { self.0 = self.0.add(other) }
    }
}

impl std::ops::Sub<usize> for StackPtr {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: usize) -> Self {
        Self(unsafe { self.0.sub(other) })
    }
}

impl std::ops::SubAssign<usize> for StackPtr {
    #[inline(always)]
    fn sub_assign(&mut self, other: usize) {
        unsafe { self.0 = self.0.sub(other) }
    }
}

impl std::ops::Sub<StackPtr> for StackPtr {
    type Output = isize;
    #[inline(always)]
    fn sub(self, other: Self) -> isize {
        unsafe { self.0.offset_from(other.0) }
    }
}

impl Index<isize> for StackPtr {
    type Output = Value;
    #[inline(always)]
    fn index(&self, index: isize) -> &Self::Output {
        unsafe { &*self.0.offset(index) }
    }
}

impl IndexMut<isize> for StackPtr {
    #[inline(always)]
    fn index_mut(&mut self, index: isize) -> &mut Self::Output {
        unsafe { &mut *self.0.offset(index) }
    }
}

impl Index<std::ops::Range<usize>> for StackPtr {
    type Output = [Value];
    #[inline(always)]
    fn index(&self, index: std::ops::Range<usize>) -> &Self::Output {
        unsafe { std::slice::from_raw_parts((*self + index.start).0, index.len()) }
    }
}

impl IndexMut<std::ops::Range<usize>> for StackPtr {
    #[inline(always)]
    fn index_mut(&mut self, index: std::ops::Range<usize>) -> &mut Self::Output {
        unsafe { std::slice::from_raw_parts_mut((*self + index.start).0, index.len()) }
    }
}

impl StackPtr {
    #[inline(always)]
    pub(super) fn as_ptr(self) -> *mut Value {
        self.0
    }

    #[inline(always)]
    pub(super) fn from(ptr: *mut Value) -> Self {
        Self(ptr)
    }

    #[inline(always)]
    pub(crate) fn as_cfp(self) -> ControlFrame {
        ControlFrame::from_ptr(self.0)
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
        stack.resize_to(stack.sp + 4);
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
