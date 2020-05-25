use crate::*;
use std::cell::RefCell;
use std::ptr::NonNull;

thread_local! (
    pub static ALLOC: RefCell<Allocator> = {
        let alloc = Allocator::new();
        RefCell::new(alloc)
    }
);

const GCBOX_SIZE: usize = 56;
const PAGE_LEN: usize = 64 * 64;
const ALIGN: usize = 0x4_0000; // 256kb

pub trait GC {
    fn mark(&self, alloc: &mut Allocator);
}

type FreeListRef = Ref<FreeList>;

struct FreeList {
    next: Option<FreeListRef>,
}

impl FreeList {
    fn new(next: FreeListRef) -> Self {
        FreeList { next: Some(next) }
    }

    fn new_null() -> Self {
        FreeList { next: None }
    }
}

pub struct Allocator {
    /// Pointer to current page.
    //buf: *mut RValue,
    /// Allocated number of objects in current page.
    used: usize,
    /// Allocation size in byte for a single arena.
    alloc_size: usize,
    /// Info for allocated pages.
    pages: Vec<(*mut RValue, [u64; 64])>,
    /// Flag for new page allocation.
    alloc_flag: bool,
    /// Counter of marked objects,
    mark_counter: usize,
    /// List of free objects.
    free: Option<FreeListRef>,
}

impl Allocator {
    pub fn new() -> Self {
        assert_eq!(56, std::mem::size_of::<RValue>());
        let mem_size = GCBOX_SIZE;
        let alloc_size = PAGE_LEN * mem_size + ALIGN - 1;
        let arena = Allocator::alloc_page(alloc_size);
        Allocator {
            //buf: arena,
            used: 0,
            alloc_size,
            pages: vec![(arena, [0; 64])],
            alloc_flag: false,
            mark_counter: 0,
            free: None,
        }
    }

    pub fn is_allocated(&self) -> bool {
        self.alloc_flag
    }

    pub fn clear_allocated(&mut self) {
        self.alloc_flag = false;
    }

    /// Clear all mark bitmaps.
    pub fn clear_mark(&mut self) {
        self.pages
            .iter_mut()
            .for_each(|(_, bitmap)| bitmap.iter_mut().for_each(|v| *v = 0));
        self.mark_counter = 0;
    }

    /// Get counter of marked objects.
    pub fn get_counter(&self) -> usize {
        self.mark_counter
    }

    /// Allocate page with `alloc_size` and `align`.
    fn alloc_page(alloc_size: usize) -> *mut RValue {
        let mut vec = Vec::<u8>::with_capacity(alloc_size);
        unsafe {
            vec.set_len(alloc_size);
        }
        let ptr = (Box::into_raw(vec.into_boxed_slice()) as *const u8 as usize + ALIGN - 1)
            & !(ALIGN - 1);
        assert_eq!(0, ptr as *const u8 as usize & (ALIGN - 1));
        ptr as *mut RValue
    }

    pub fn gc<T: GC>(&mut self, root: &T) {
        self.clear_mark();
        root.mark(self);
        eprintln!("marked: {}", self.get_counter());
        self.sweep();
        self.clear_allocated();
        //self.print_mark();
    }

    /// If object is already marked, return true.
    /// If not yet, mark it and return false.
    pub fn mark(&mut self, ptr: &RValue) -> bool {
        let ptr = ptr as *const RValue as usize;
        let page_ptr = ptr & !(ALIGN - 1);
        let page_info = self
            .pages
            .iter_mut()
            .find(|(p, _)| *p == page_ptr as *mut RValue)
            .unwrap_or_else(|| panic!("The ptr is not in heap pages."));
        let offset = ptr - page_ptr;
        assert_eq!(0, offset % GCBOX_SIZE);
        let index = offset / GCBOX_SIZE;
        assert!(index < PAGE_LEN);
        let bit_mask = 1 << (index % 64);
        let word = index / 64;
        let bitmap = &mut page_info.1[word];
        let is_marked = (*bitmap & bit_mask) != 0;
        *bitmap |= bit_mask;
        if !is_marked {
            self.mark_counter += 1;
        }
        is_marked
    }

    pub fn sweep(&mut self) {
        let mut c = 0;

        for (page_ptr, bitmap) in self.pages.iter() {
            for (i, map) in bitmap.iter().enumerate() {
                let mut map = *map;
                for bit in 0..64 {
                    if map & 1 == 0 {
                        unsafe {
                            let p = page_ptr.add(i * 64 + bit) as *mut FreeList;
                            let free = match self.free {
                                None => FreeList::new_null(),
                                Some(f) => FreeList::new(f),
                            };
                            std::ptr::write(p, free);
                            self.free = Some(FreeListRef::from_ptr(p));
                            //let p = page_ptr.add(i * 64 + bit);
                            //std::ptr::write(p, RValue::new_flonum(0.5));
                        }
                        c += 1
                    }
                    map >>= 1;
                }
            }
        }
        eprintln!("sweep: {}", c);
        c = 0;
        let mut free = self.free;
        loop {
            match free {
                Some(f) => {
                    free = f.next;
                    c += 1;
                }
                None => break,
            };
        }
        eprintln!("free list: {}", c);
    }

    pub fn print_mark(&self) {
        self.pages.iter().for_each(|(_, bitmap)| {
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
        if self.free.is_some() {
            let ret = self.free.unwrap();
            self.free = ret.next;
            eprintln!("free_alloc");
            return ret.as_ptr() as *mut RValue;
        }
        let ptr = unsafe {
            let page = self.pages.last().unwrap().0;
            let ptr = page.add(self.used);
            std::ptr::write(ptr, data);
            ptr
        };
        eprintln!("wm_alloc: {:?}", self.used);
        self.used += 1;
        if self.used >= 2000 {
            self.alloc_flag = true;
        }
        if self.used >= PAGE_LEN {
            eprintln!("alloc new page");
            let page_ptr = Allocator::alloc_page(self.alloc_size);
            self.used = 0;
            self.pages.push((page_ptr, [0; 64]));
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

            100_000.times {
                Vec.new
            }
        "#;
        let res = vm.run(PathBuf::from("test"), &program, None);
        //vm.gc();
        //vm.print_bitmap();
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
