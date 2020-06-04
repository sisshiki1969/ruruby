use crate::*;
use std::cell::RefCell;
use std::sync::Mutex;

lazy_static! {
    pub static ref ALLOC: Mutex<Allocator> = {
        let alloc = Allocator::new();
        Mutex::new(alloc)
    };
}

thread_local! {
    pub static ALLOC_THREAD: RefCell<AllocThread> = {
        RefCell::new(AllocThread {
            allocated:0,
            alloc_flag:false
        })
    };
}

const OFFSET: usize = 0;
const GCBOX_SIZE: usize = std::mem::size_of::<GCBox>();
const PAGE_LEN: usize = 64 * 64;
const ALIGN: usize = 0x4_0000; // 2^18 = 256kb
const ALLOC_SIZE: usize = PAGE_LEN * GCBOX_SIZE + ALIGN - 1;
const GC_THRESHOLD: usize = 1024;

pub trait GC {
    fn mark(&self, alloc: &mut Allocator);
}

struct GCBox {
    inner: RValue,
    next: Option<GCBoxRef>,
}

impl GCBox {
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
    pages: Vec<PageInfo>,
    /// Counter of marked objects,
    mark_counter: usize,
    /// List of free objects.
    free: Option<GCBoxRef>,
}

pub struct AllocThread {
    allocated: usize,
    alloc_flag: bool,
}

impl AllocThread {
    pub fn is_allocated(&self) -> bool {
        self.alloc_flag
    }
}

struct PageInfo {
    ptr: GCBoxRef,
    bitmap: [u64; 64],
}

impl Allocator {
    pub fn new() -> Self {
        #[cfg(debug_assertions)]
        {
            assert_eq!(56, std::mem::size_of::<RValue>());
            assert_eq!(64, GCBOX_SIZE);
            let gc_box = GCBox {
                inner: RValue::new_invalid(),
                next: None,
            };
            assert_eq!(
                OFFSET,
                gc_box.inner_ptr() as usize - &gc_box as *const GCBox as usize
            );
        }
        let ptr = Allocator::alloc_page(ALLOC_SIZE);
        Allocator {
            used: 0,
            allocated: 0,
            pages: vec![PageInfo {
                ptr,
                bitmap: [0; 64],
            }],
            mark_counter: 0,
            free: None,
        }
    }

    /// Clear all mark bitmaps.
    pub fn clear_mark(&mut self) {
        self.pages
            .iter_mut()
            .for_each(|pinfo| pinfo.bitmap.iter_mut().for_each(|v| *v = 0));
        self.mark_counter = 0;
    }

    /// Get counter of marked objects.
    pub fn get_counter(&self) -> usize {
        self.mark_counter
    }

    /// Allocate page with `alloc_size` and `align`.
    fn alloc_page(alloc_size: usize) -> GCBoxRef {
        let mut vec = Vec::<u8>::with_capacity(alloc_size);
        unsafe { vec.set_len(alloc_size) };
        let ptr = (Box::into_raw(vec.into_boxed_slice()) as *const u8 as usize + ALIGN - 1)
            & !(ALIGN - 1);
        let ptr = ptr as *mut GCBox;
        #[cfg(debug_assertions)]
        {
            assert_eq!(0, ptr as *const u8 as usize & (ALIGN - 1));
            eprintln!("page allocated: {:?}", ptr);
        }
        GCBoxRef::from_ptr(ptr)
    }

    pub fn gc(&mut self, root: &Globals) {
        #[cfg(debug_assertions)]
        {
            eprintln!("--GC start thread:{:?}", std::thread::current().id());
            eprintln!("allocated: {}", self.allocated);
            eprintln!("used in current page: {}", self.used);
        }
        self.clear_mark();
        root.mark(self);
        #[cfg(debug_assertions)]
        {
            eprintln!("marked: {}", self.get_counter());
        }
        self.sweep();
        ALLOC_THREAD.with(|m| {
            m.borrow_mut().alloc_flag = false;
        });
        #[cfg(debug_assertions)]
        {
            self.print_mark();
            for vm in &root.fibers {
                vm.dump_values();
            }
            eprintln!("--GC completed")
        }
    }

    /// Allocate object.
    pub fn alloc(&mut self, data: RValue) -> *mut RValue {
        self.allocated += 1;
        ALLOC_THREAD.with(|m| {
            let mut m = m.borrow_mut();
            m.allocated += 1;
            m.alloc_flag = m.allocated % GC_THRESHOLD == 0;
            #[cfg(debug_assertions)]
            { /*
                     if m.alloc_flag {
                         eprintln!("prepare GC... {:?}", std::thread::current().id());
                     }
                 */
            }
        });

        match self.free {
            Some(mut gcbox) => {
                // Allocate from the free list.
                self.free = gcbox.next;
                gcbox.next = None;
                gcbox.inner = data;
                return gcbox.inner_ptr();
            }
            None => {}
        }

        let mut gcbox = if self.used == PAGE_LEN {
            // Allocate new page.
            let page_ptr = Allocator::alloc_page(ALLOC_SIZE);
            self.used = 0;
            self.pages.push(PageInfo {
                ptr: page_ptr,
                bitmap: [0; 64],
            });
            page_ptr
        } else {
            // Bump allocation.
            let ptr = unsafe { self.pages.last().unwrap().ptr.as_ptr().add(self.used) };
            GCBoxRef::from_ptr(ptr)
        };
        //eprintln!("wm_alloc: {:?}", self.used);

        self.used += 1;
        gcbox.next = None;
        gcbox.inner = data;
        gcbox.inner_ptr()
    }

    /// Mark object.
    /// If object is already marked, return true.
    /// If not yet, mark it and return false.
    pub fn mark(&mut self, ptr: &RValue) -> bool {
        let ptr = (ptr as *const RValue as usize - OFFSET) as *const GCBox as *mut GCBox;
        self.mark_ptr(ptr)
    }

    /// Mark object.
    /// If object is already marked, return true.
    /// If not yet, mark it and return false.
    fn mark_ptr(&mut self, ptr: *mut GCBox) -> bool {
        let ptr = ptr as *const GCBox as usize;
        let page_ptr = ptr & !(ALIGN - 1);
        let page_info = self
            .pages
            .iter_mut()
            .find(|pinfo| pinfo.ptr.as_ptr() == page_ptr as *mut GCBox)
            .unwrap_or_else(|| {
                panic!("The ptr is not in heap pages. {:?}", page_ptr as *mut GCBox)
            });
        let offset = ptr - page_ptr;
        let index = offset / GCBOX_SIZE;
        #[cfg(debug_assertions)]
        {
            assert_eq!(0, offset % GCBOX_SIZE);
            assert!(index < PAGE_LEN);
        }
        let bit_mask = 1 << (index % 64);
        let bitmap = &mut page_info.bitmap[index / 64];
        let is_marked = (*bitmap & bit_mask) != 0;
        *bitmap |= bit_mask;
        if !is_marked {
            self.mark_counter += 1;
        }
        is_marked
    }

    pub fn sweep(&mut self) {
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
        let pinfo = self.pages.last().unwrap();
        let mut ptr = pinfo.ptr.as_ptr();
        for map in pinfo.bitmap.iter().take(self.used / 64) {
            let mut map = *map;
            for _ in 0..64 {
                if map & 1 == 0 {
                    unsafe {
                        (*ptr).next = self.free;
                        (*ptr).inner.free();
                        (*ptr).inner = RValue::new_invalid();
                    }
                    self.free = Some(GCBoxRef::from_ptr(ptr));
                    c += 1;
                }
                ptr = unsafe { ptr.add(1) };
                map >>= 1;
            }
        }

        let i = self.used / 64;
        let bit = self.used % 64;
        let mut map = pinfo.bitmap[i];
        for _ in 0..bit {
            if map & 1 == 0 {
                unsafe {
                    (*ptr).next = self.free;
                    (*ptr).inner.free();
                    (*ptr).inner = RValue::new_invalid();
                }
                self.free = Some(GCBoxRef::from_ptr(ptr));
                c += 1;
            }
            ptr = unsafe { ptr.add(1) };
            map >>= 1;
        }

        for pinfo in self.pages[0..self.pages.len() - 1].iter() {
            let mut ptr = pinfo.ptr.as_ptr();
            for map in pinfo.bitmap.iter() {
                let mut map = *map;
                for _ in 0..64 {
                    if map & 1 == 0 {
                        unsafe {
                            (*ptr).next = self.free;
                            (*ptr).inner.free();
                            (*ptr).inner = RValue::new_invalid();
                        }
                        self.free = Some(GCBoxRef::from_ptr(ptr));
                        c += 1;
                    }
                    ptr = unsafe { ptr.add(1) };
                    map >>= 1;
                }
            }
        }
        #[cfg(debug_assertions)]
        {
            eprintln!("sweep: {}", c);
            eprintln!("free list: {}", self.check_free_list());
        }
    }

    // For debug
    #[allow(dead_code)]
    fn check_ptr(&self, ptr: *mut GCBox) {
        let ptr = ptr as *const GCBox as usize;
        let page_ptr = ptr & !(ALIGN - 1);
        self.pages
            .iter()
            .find(|pinfo| pinfo.ptr.as_ptr() == page_ptr as *mut GCBox)
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

    #[allow(dead_code)]
    pub fn print_mark(&self) {
        self.pages.iter().for_each(|pinfo| {
            let mut i = 0;
            pinfo.bitmap.iter().for_each(|m| {
                eprint!("{:016x} ", m.reverse_bits());
                if i % 8 == 7 {
                    eprintln!("");
                }
                i += 1;
            });
            eprintln!("");
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use std::path::PathBuf;

    #[test]
    fn gc_test() {
        let mut vm = VMRef::new(VM::new());
        vm.clone().globals.fibers.push(vm);
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
