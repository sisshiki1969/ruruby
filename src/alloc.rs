use crate::*;
use std::cell::RefCell;

thread_local! (
    pub static ALLOC: RefCell<Allocator> = {
        let alloc = Allocator::new();
        RefCell::new(alloc)
    }
);

const LEN: usize = 64 * 64;
const ALIGN: usize = 0x4_0000; // 256kb

pub trait GC {
    fn mark(&self, alloc: &mut Allocator);
}

pub struct Allocator {
    /// Pointer to current arena.
    buf: *mut RValue,
    /// Allocated number of objects.
    used: usize,
    /// Allocation size in byte for a single arena.
    alloc_size: usize,
    /// Info for allocated arenas.
    arena: Vec<(*mut RValue, [u64; 64])>,
    /// Flag for new arena allocation.
    alloc_flag: bool,
}

impl Allocator {
    pub fn new() -> Self {
        assert_eq!(56, std::mem::size_of::<RValue>());
        let mem_size = 56;
        let alloc_size = LEN * mem_size + ALIGN - 1;
        let arena = Allocator::arena(alloc_size, ALIGN - 1);
        Allocator {
            buf: arena,
            used: 0,
            alloc_size,
            arena: vec![(arena, [0; 64])],
            alloc_flag: false,
        }
    }

    /// Allocate arena with `alloc_size` and `align`.
    fn arena(alloc_size: usize, align: usize) -> *mut RValue {
        let mut vec = Vec::<u8>::with_capacity(alloc_size);
        unsafe {
            vec.set_len(alloc_size);
        }
        let ptr = (Box::into_raw(vec.into_boxed_slice()) as *const u8 as usize + align) & !align;
        assert_eq!(0, ptr as *const u8 as usize & align);
        ptr as *mut RValue
    }

    pub fn is_allocated(&self) -> bool {
        self.alloc_flag
    }

    pub fn clear_allocated(&mut self) {
        self.alloc_flag = false;
    }

    /// If object is already marked, return true.
    /// If not yet, mark it and return false.
    pub fn mark(&mut self, ptr: &RValue) -> bool {
        let ptr = ptr as *const RValue as usize;
        let arena = ptr & !(ALIGN - 1);
        let arena_info = self
            .arena
            .iter_mut()
            .find(|(ptr, _)| *ptr == arena as *mut RValue)
            .unwrap_or_else(|| panic!());
        //contains(&(arena as *mut RValue));
        assert!(ptr >= arena);
        let offset = ptr - arena;
        assert_eq!(0, offset % 56);
        let index = offset / 56;
        assert!(index < LEN);
        let bit_mask = 1 << (index % 64);
        let word = index / 64;
        let bitmap = &mut arena_info.1[word];
        let is_marked = (*bitmap & bit_mask) != 0;
        *bitmap |= bit_mask;
        is_marked
    }

    pub fn clear_mark(&mut self) {
        self.arena
            .iter_mut()
            .for_each(|(_, bitmap)| bitmap.iter_mut().for_each(|v| *v = 0));
    }

    pub fn print_mark(&self) {
        self.arena.iter().for_each(|(_, bitmap)| {
            let mut i = 0;
            bitmap.iter().for_each(|m| {
                eprint!("{:016x} ", m.reverse_bits());
                if i % 8 == 7 {
                    eprintln!("");
                }
                i += 1;
            });
            eprintln!("");
        });
    }

    /// Allocate object.
    pub fn alloc(&mut self, data: RValue) -> *mut RValue {
        let ptr = unsafe {
            let ptr = self.buf.add(self.used);
            std::ptr::write(ptr, data);
            ptr
        };
        self.used += 1;
        //eprintln!("alloc: {:?}", self.used);
        if self.used >= LEN {
            let arena = Allocator::arena(self.alloc_size, ALIGN - 1);
            self.used = 0;
            self.buf = arena;
            self.arena.push((arena, [0; 64]));
            self.alloc_flag = true;
        }
        ptr
    }

    #[allow(dead_code)]
    unsafe fn free(&mut self, raw: *mut RValue) {
        let s = std::slice::from_raw_parts_mut(raw as *mut u8, self.alloc_size);
        let _ = Box::from_raw(s);
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use std::path::PathBuf;

    #[test]
    fn gc_test() {
        let mut vm = VM::new();
        let program = r#"
            class Vec
                def initialize
                    @x = 100
                    @y = 200
                end
            end

            1000_000.times {
                Vec.new
            }
        "#;
        let res = vm.run(PathBuf::from("test"), &program, None);
        vm.gc();
        vm.print_bitmap();
        match res {
            Ok(_) => {}
            Err(err) => {
                err.show_err();
                err.show_loc(0);
                panic!("Got error: {:?}", err);
            }
        };
    }
}
