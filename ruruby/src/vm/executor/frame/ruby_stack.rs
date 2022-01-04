use super::impl_ptr_ops;
use crate::{ControlFrame, LocalFrame, Value, CF};
use std::ops::{Index, IndexMut};
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
pub(crate) struct RubyStack {
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

impl RubyStack {
    /// Allocate new `RubyStack`.
    pub(crate) fn new() -> Self {
        let mut inner = unsafe { Box::new_uninit_slice(VM_STACK_SIZE).assume_init() };
        let sp = StackPtr::from(inner.as_mut_ptr());
        Self {
            sp,
            buf: Pin::from(inner),
        }
    }

    pub(crate) fn bottom(&self) -> StackPtr {
        StackPtr::from(self.buf.as_ptr() as _)
    }

    pub(crate) fn check_boundary(&self, p: *mut Value) -> Option<usize> {
        let ptr = self.buf.as_ptr() as *mut Value;
        unsafe {
            if ptr <= p && p < ptr.add(VM_STACK_SIZE) {
                Some(p.offset_from(ptr as *const _) as usize)
            } else {
                None
            }
        }
    }

    /// Get length of stack.
    /// This is as same as the index of SP in the stack.
    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        let len = self.sp - self.bottom();
        assert!(len >= 0);
        len as usize
    }

    /// Resize the stack so that len is equal to new_len.
    /// If new_len is greater than len, the stack is extended by the difference, with each additional slot filled with Nil.
    /// If new_len is less than len, the stack is simply truncated.
    pub(crate) fn resize_to(&mut self, new_sp: StackPtr) {
        debug_assert!(new_sp <= self.sp + VM_STACK_SIZE);
        if new_sp > self.sp {
            let mut p = self.sp;
            for _ in 0..(new_sp - self.sp) as usize {
                p[0] = Value::nil();
                p += 1;
            }
        }
        self.sp = new_sp;
    }

    #[inline(always)]
    pub(crate) fn grow(&mut self, offset: usize) {
        debug_assert!(self.len() + offset <= VM_STACK_SIZE);
        for _ in 0..offset {
            self.sp[0] = Value::nil();
            self.sp += 1;
        }
    }

    pub(crate) fn remove(&mut self, p: StackPtr) -> Value {
        let v = p[0];
        unsafe { std::ptr::copy((p + 1).0, p.0, (self.sp - p - 1) as usize) };
        self.sp -= 1;
        v
    }

    pub(crate) fn insert(&mut self, mut p: StackPtr, element: Value) {
        self.sp += 1;
        unsafe { std::ptr::copy(p.0, (p + 1).0, (self.sp - p - 1) as usize) };
        p[0] = element;
    }

    #[inline(always)]
    pub(crate) fn push(&mut self, val: Value) {
        debug_assert!(self.len() != VM_STACK_SIZE);
        self.sp[0] = val;
        self.sp += 1;
    }

    #[inline(always)]
    pub(crate) fn pop(&mut self) -> Value {
        debug_assert!(self.len() != 0);
        self.sp -= 1;
        self.sp[0]
    }

    #[inline(always)]
    pub(crate) fn pop2(&mut self) -> (Value, Value) {
        debug_assert!(self.len() >= 2);
        self.sp -= 2;
        let ptr = self.sp;
        (ptr[0], ptr[1])
    }

    #[cfg(feature = "trace")]
    #[inline(always)]
    pub(crate) fn last(&self) -> Value {
        debug_assert!(self.len() != 0);
        self.sp[-1]
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<Value> {
        let len = self.len();
        self.buf[0..len].iter()
    }

    pub(crate) fn extend_from_slice(&mut self, src: &[Value]) {
        let src_len = src.len();
        self.sp[0..src_len].copy_from_slice(src);
        self.sp += src_len;
    }

    pub(crate) fn extend_from_within_ptr(&mut self, src: StackPtr, len: usize) {
        let slice = &src[0..len];
        self.sp[0..len].copy_from_slice(slice);
        self.sp += len;
    }

    pub(crate) fn stack_copy_within(ptr: StackPtr, src: std::ops::Range<usize>, dest: usize) {
        unsafe { std::ptr::copy((ptr + src.start).0, (ptr + dest).0, src.len()) };
    }

    #[inline(always)]
    pub(crate) fn as_mut_ptr(&self) -> *mut Value {
        self.buf.as_ptr() as _
    }
}

#[derive(Debug, Clone, Copy, PartialEq, std::cmp::PartialOrd)]
pub struct StackPtr(*mut Value);

impl_ptr_ops!(StackPtr);

/*impl std::default::Default for StackPtr {
    #[inline(always)]
    fn default() -> Self {
        StackPtr(std::ptr::null_mut())
    }
}*/

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
        *self = *self + other
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
        *self = *self - other
    }
}

impl std::ops::Sub<StackPtr> for StackPtr {
    type Output = isize;
    #[inline(always)]
    fn sub(self, other: Self) -> isize {
        unsafe { self.0.offset_from(other.0) }
    }
}

/*impl Index<isize> for StackPtr {
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
}*/

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
    pub(crate) fn as_ptr(self) -> *mut Value {
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

    #[inline(always)]
    pub(crate) fn as_lfp(self) -> LocalFrame {
        LocalFrame::from_ptr(self.0)
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
        //assert_eq!(Value::fixnum(42), stack.last());
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

    macro_rules! i {
        ($val:expr) => {
            Value::fixnum($val)
        };
    }

    #[test]
    fn stack2() {
        let mut stack = RubyStack::new();
        let mut top = stack.bottom();
        stack.push(i!(5));
        stack.push(i!(7));
        assert_eq!(2, stack.len());
        assert_eq!(5, top[0].as_fnum());
        assert_eq!(7, top[1].as_fnum());
        stack.resize_to(stack.sp + 2);
        assert_eq!(5, top[0].as_fnum());
        assert_eq!(7, top[1].as_fnum());
        assert_eq!(Value::nil(), top[2]);
        assert_eq!(Value::nil(), top[3]);
        assert_eq!(4, stack.len());
        top[3] = Value::fixnum(99);
        assert_eq!(4, stack.len());
        assert_eq!(5, top[0].as_fnum());
        assert_eq!(7, top[1].as_fnum());
        assert_eq!(Value::nil(), top[2]);
        assert_eq!(99, top[3].as_fnum());
        stack.extend_from_slice(&[i!(34), i!(56)]);
        assert_eq!(6, stack.len());
        assert_eq!(5, top[0].as_fnum());
        assert_eq!(7, top[1].as_fnum());
        assert_eq!(Value::nil(), top[2]);
        assert_eq!(99, top[3].as_fnum());
        assert_eq!(34, top[4].as_fnum());
        assert_eq!(56, top[5].as_fnum());
        RubyStack::stack_copy_within(stack.sp - 4, 1..4, 0);
        assert_eq!(6, stack.len());
        assert_eq!(5, top[0].as_fixnum().unwrap());
        assert_eq!(7, top[1].as_fixnum().unwrap());
        assert_eq!(99, top[2].as_fixnum().unwrap());
        assert_eq!(34, top[3].as_fixnum().unwrap());
        assert_eq!(56, top[4].as_fixnum().unwrap());
        assert_eq!(56, top[5].as_fixnum().unwrap());
        stack.remove(top + 4);
        assert_eq!(5, stack.len());
        assert_eq!(5, top[0].as_fixnum().unwrap());
        assert_eq!(7, top[1].as_fixnum().unwrap());
        assert_eq!(99, top[2].as_fixnum().unwrap());
        assert_eq!(34, top[3].as_fixnum().unwrap());
        assert_eq!(56, top[4].as_fixnum().unwrap());
        stack.remove(top + 1);
        assert_eq!(4, stack.len());
        assert_eq!(5, top[0].as_fixnum().unwrap());
        assert_eq!(99, top[1].as_fixnum().unwrap());
        assert_eq!(34, top[2].as_fixnum().unwrap());
        assert_eq!(56, top[3].as_fixnum().unwrap());
        stack.insert(top + 1, Value::fixnum(42));
        assert_eq!(5, stack.len());
        assert_eq!(5, top[0].as_fixnum().unwrap());
        assert_eq!(42, top[1].as_fixnum().unwrap());
        assert_eq!(99, top[2].as_fixnum().unwrap());
        assert_eq!(34, top[3].as_fixnum().unwrap());
        assert_eq!(56, top[4].as_fixnum().unwrap());
    }

    #[test]
    fn stack3() {
        let mut stack = RubyStack::new();
        let top = stack.bottom();
        stack.push(Value::fixnum(3));
        stack.grow(2);
        assert_eq!(3, stack.len());
        assert_eq!(3, top[0].as_fixnum().unwrap());
        assert_eq!(Value::nil(), top[1]);
        assert_eq!(Value::nil(), top[2]);
    }
}
