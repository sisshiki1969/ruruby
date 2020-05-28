use crate::*;
use std::cell::RefCell;

thread_local! (
    pub static ALLOC: RefCell<Allocator> = {
        let alloc = Allocator::new();
        RefCell::new(alloc)
    }
);

const OFFSET: usize = 0;
const GCBOX_SIZE: usize = std::mem::size_of::<GCBox>();
const PAGE_LEN: usize = 64 * 64;
const ALIGN: usize = 0x4_0000; // 256kb
const ALLOC_SIZE: usize = PAGE_LEN * GCBOX_SIZE + ALIGN - 1;

pub trait GC {
    fn mark(&self, alloc: &mut Allocator);
}

struct GCBox {
    inner: RValue,
    next: Option<GCBoxRef>,
}

impl GCBox {
    fn new(next: Option<GCBoxRef>) -> Self {
        GCBox {
            inner: RValue::new_ordinary(Value::nil()),
            next,
        }
    }

    fn new_rvalue(data: RValue) -> Self {
        GCBox {
            inner: data,
            next: None,
        }
    }

    fn inner_ptr(&self) -> *mut RValue {
        &self.inner as *const RValue as *mut RValue
    }
}

type GCBoxRef = Ref<GCBox>;

pub struct Allocator {
    /// Allocated number of objects in current page.
    used: usize,
    /// Total allocated objects.
    allocated: usize,
    /// Info for allocated pages.
    pages: Vec<(GCBoxRef, [u64; 64])>,
    /// Flag for new page allocation.
    alloc_flag: bool,
    /// Counter of marked objects,
    mark_counter: usize,
    /// List of free objects.
    free: Option<GCBoxRef>,
}

impl Allocator {
    pub fn new() -> Self {
        assert_eq!(56, std::mem::size_of::<RValue>());
        assert_eq!(64, GCBOX_SIZE);
        let gc_box = GCBox::new(None);
        assert_eq!(
            OFFSET,
            gc_box.inner_ptr() as usize - &gc_box as *const GCBox as usize
        );
        let page_ptr = Allocator::alloc_page(ALLOC_SIZE);
        Allocator {
            //buf: arena,
            used: 0,
            allocated: 0,
            pages: vec![(GCBoxRef::from_ptr(page_ptr), [0; 64])],
            alloc_flag: false,
            mark_counter: 0,
            free: None,
        }
    }

    pub fn is_allocated(&self) -> bool {
        self.alloc_flag
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
    fn alloc_page(alloc_size: usize) -> *mut GCBox {
        let mut vec = Vec::<u8>::with_capacity(alloc_size);
        unsafe {
            vec.set_len(alloc_size);
        }
        let ptr = (Box::into_raw(vec.into_boxed_slice()) as *const u8 as usize + ALIGN - 1)
            & !(ALIGN - 1);
        //assert_eq!(0, ptr as *const u8 as usize & (ALIGN - 1));
        #[cfg(features = "verbose")]
        eprintln!("page allocated: {:?}", ptr as *mut GCBox);
        ptr as *mut GCBox
    }

    pub fn gc(&mut self, root: &mut VM) {
        #[cfg(features = "verbose")]
        {
            eprintln!("--GC start");
            eprintln!("allocated: {}", self.allocated);
        }
        self.clear_mark();
        root.mark(self);
        #[cfg(features = "verbose")]
        eprintln!("marked: {}", self.get_counter());
        self.sweep(root);
        self.alloc_flag = false;
        //self.print_mark();
        #[cfg(features = "verbose")]
        eprintln!("--GC completed")
    }

    /// Allocate object.
    pub fn alloc(&mut self, data: RValue) -> *mut RValue {
        match self.free {
            Some(mut free) => {
                self.free = free.next;
                free.next = None;
                free.inner = data;
                self.allocated += 1;
                return free.inner_ptr();
            }
            None => {}
        }
        let ptr = unsafe {
            let ptr = self.pages.last().unwrap().0.as_ptr().add(self.used);
            *ptr = GCBox::new_rvalue(data);
            (*ptr).inner_ptr()
        };
        //eprintln!("wm_alloc: {:?}", self.used);
        self.used += 1;
        self.allocated += 1;

        self.alloc_flag = self.allocated % 1024 == 0;

        if self.used >= PAGE_LEN {
            let page_ptr = Allocator::alloc_page(ALLOC_SIZE);
            self.used = 0;
            self.pages.push((GCBoxRef::from_ptr(page_ptr), [0; 64]));
        }
        ptr
    }

    /// If object is already marked, return true.
    /// If not yet, mark it and return false.
    pub fn mark(&mut self, ptr: &RValue) -> bool {
        let ptr = (ptr as *const RValue as usize - OFFSET) as *const GCBox as *mut GCBox;
        self.mark_ptr(ptr)
    }

    fn mark_ptr(&mut self, ptr: *mut GCBox) -> bool {
        let ptr = ptr as *const GCBox as usize;
        let page_ptr = ptr & !(ALIGN - 1);
        let page_info = self
            .pages
            .iter_mut()
            .find(|(p, _)| p.as_ptr() == page_ptr as *mut GCBox)
            .unwrap_or_else(|| panic!("The ptr is not in heap pages."));
        let offset = ptr - page_ptr;
        //assert_eq!(0, offset % GCBOX_SIZE);
        let index = offset / GCBOX_SIZE;
        //assert!(index < PAGE_LEN);
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

    pub fn sweep(&mut self, _vm: &mut VM) {
        let mut free = self.free;
        loop {
            match free {
                Some(f) => {
                    if self.mark_ptr(f.as_ptr()) {
                        panic!("Marked object in free list.")
                    };
                    free = f.next;
                }
                None => break,
            };
        }

        #[allow(unused_variables)]
        let mut c = 0;

        let (page_ptr, bitmap) = self.pages.last().unwrap();
        for (i, map) in bitmap.iter().take(self.used / 64).enumerate() {
            let mut map = *map;
            for bit in 0..64 {
                if map & 1 == 0 {
                    let ptr = unsafe {
                        let ptr = page_ptr.as_ptr().add(i * 64 + bit);
                        (*ptr).next = self.free;
                        let dummy = RValue::new_flonum(2.5);
                        std::mem::replace(&mut (*ptr).inner, dummy);
                        ptr
                    };
                    self.free = Some(GCBoxRef::from_ptr(ptr));
                    c += 1
                }
                map >>= 1;
            }
        }

        for (page_ptr, bitmap) in self.pages[0..self.pages.len() - 1].iter() {
            for (i, map) in bitmap.iter().enumerate() {
                let mut map = *map;
                for bit in 0..64 {
                    if map & 1 == 0 {
                        let ptr = unsafe {
                            let ptr = page_ptr.as_ptr().add(i * 64 + bit);
                            (*ptr).next = self.free;
                            let dummy = RValue::new_flonum(2.5);
                            std::mem::replace(&mut (*ptr).inner, dummy);
                            ptr
                        };
                        self.free = Some(GCBoxRef::from_ptr(ptr));
                        c += 1
                    }
                    map >>= 1;
                }
            }
        }
        #[cfg(features = "verbose")]
        eprintln!("sweep: {}", c);
        //eprintln!("free list: {}", self.check_free_list());
    }

    // For debug
    #[allow(dead_code)]
    fn check_ptr(&self, ptr: *mut GCBox) {
        let ptr = ptr as *const GCBox as usize;
        let page_ptr = ptr & !(ALIGN - 1);
        self.pages
            .iter()
            .find(|(p, _)| p.as_ptr() == page_ptr as *mut GCBox)
            .unwrap_or_else(|| panic!("The ptr is not in heap pages."));
    }

    #[allow(dead_code)]
    fn check_free_list(&self) -> usize {
        let mut c = 0;
        let mut free = self.free;
        loop {
            match free {
                Some(f) => {
                    self.check_ptr(f.as_ptr());
                    free = f.next;
                }
                None => break,
            };
            c += 1;
        }
        c
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

    #[allow(dead_code)]
    unsafe fn free(&mut self, raw: *mut RValue) {
        let s = std::slice::from_raw_parts_mut(raw as *mut u8, ALLOC_SIZE);
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
