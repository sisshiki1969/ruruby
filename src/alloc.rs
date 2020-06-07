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

const GCBOX_SIZE: usize = std::mem::size_of::<GCBox<RValue>>();
const PAGE_LEN: usize = 64 * 64;
const ALIGN: usize = 0x4_0000; // 2^18 = 256kb
const ALLOC_SIZE: usize = PAGE_LEN * GCBOX_SIZE;

pub trait GC {
    fn mark(&self, alloc: &mut Allocator);
}

#[derive(Debug, Clone)]
pub struct GCBox<T: GC> {
    inner: T,
    next: Option<GCBoxRef<T>>,
}

impl<T: GC> GCBox<T> {
    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl GCBox<RValue> {
    pub fn gc_mark(&self, alloc: &mut Allocator) {
        if alloc.mark(self) {
            return;
        };
        self.inner.mark(alloc);
    }
}

impl<T: GC> std::ops::Deref for GCBox<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: GC> std::ops::DerefMut for GCBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

type GCBoxRef<T> = Ref<GCBox<T>>;

pub struct Allocator {
    /// Allocated number of objects in current page.
    used: usize,
    /// Total allocated objects.
    allocated: usize,
    /// Total sweeped objects.
    sweeped: usize,
    /// Info for allocated pages.
    pages: Vec<PageInfo<RValue>>,
    /// Counter of marked objects,
    mark_counter: usize,
    /// List of free objects.
    free: Option<GCBoxRef<RValue>>,
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

struct PageInfo<T: GC> {
    ptr: GCBoxRef<T>,
    bitmap: [u64; 64],
}

impl Allocator {
    pub fn new() -> Self {
        assert_eq!(56, std::mem::size_of::<RValue>());
        assert_eq!(64, GCBOX_SIZE);
        let ptr = Allocator::alloc_page();
        Allocator {
            used: 0,
            allocated: 0,
            sweeped: 0,
            pages: vec![PageInfo {
                ptr: GCBoxRef::from_ptr(ptr),
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

    /// Allocate page with `alloc_size` and `align`.
    fn alloc_page() -> *mut GCBox<RValue> {
        use std::alloc::{alloc, Layout};
        let layout = Layout::from_size_align(ALLOC_SIZE, ALIGN).unwrap();
        let ptr = unsafe { alloc(layout) };

        #[cfg(debug_assertions)]
        {
            assert_eq!(0, ptr as *const u8 as usize & (ALIGN - 1));
            eprintln!("page allocated: {:?}", ptr);
        }
        ptr as *mut GCBox<RValue>
    }

    /// Allocate object.
    pub fn alloc(&mut self, data: RValue) -> *mut GCBox<RValue> {
        self.allocated += 1;
        ALLOC_THREAD.with(|m| {
            let mut m = m.borrow_mut();
            m.allocated += 1;
            m.alloc_flag = m.allocated % 1024 == 0;
        });

        match self.free {
            Some(gcbox) => {
                // Allocate from the free list.
                self.free = gcbox.next;
                #[cfg(debug_assertions)]
                assert_eq!(gcbox.inner, RValue::new_invalid());
                unsafe {
                    std::ptr::write(
                        gcbox.as_ptr(),
                        GCBox {
                            inner: data,
                            next: None,
                        },
                    );
                }
                return gcbox.as_ptr();
            }
            None => {}
        }

        let gcbox = if self.used == PAGE_LEN {
            // Allocate new page.
            let ptr = Allocator::alloc_page();
            self.used = 1;
            self.pages.push(PageInfo {
                ptr: GCBoxRef::from_ptr(ptr),
                bitmap: [0; 64],
            });
            ptr
        } else {
            // Bump allocation.
            let ptr = unsafe { self.pages.last().unwrap().ptr.as_ptr().add(self.used) };
            self.used += 1;
            ptr
        };
        #[cfg(debug_assertions)]
        {
            assert!(self.used <= PAGE_LEN);
            assert!(0 < self.used);
        }

        unsafe {
            std::ptr::write(
                gcbox,
                GCBox {
                    inner: data,
                    next: None,
                },
            );
        }
        gcbox
    }

    pub fn gc(&mut self, root: &Globals) {
        #[cfg(debug_assertions)]
        {
            eprintln!("--GC start thread:{:?}", std::thread::current().id());
            eprintln!("allocated: {}", self.allocated);
            eprintln!("used in current page: {}", self.used);
            eprintln!("allocated pages: {}", self.pages.len());
        }
        self.clear_mark();
        root.mark(self);
        #[cfg(debug_assertions)]
        {
            eprintln!("marked: {}", self.mark_counter);
        }
        self.sweep();
        #[cfg(debug_assertions)]
        {
            eprintln!("sweeed: {}", self.sweeped);
        }
        ALLOC_THREAD.with(|m| {
            m.borrow_mut().alloc_flag = false;
        });
        #[cfg(debug_assertions)]
        {
            eprintln!("--GC completed");
        }
    }

    /// Mark object.
    /// If object is already marked, return true.
    /// If not yet, mark it and return false.
    pub fn mark(&mut self, ptr: &GCBox<RValue>) -> bool {
        let ptr = ptr as *const GCBox<RValue> as *mut GCBox<RValue>;
        self.mark_ptr(ptr)
    }

    /// Mark object.
    /// If object is already marked, return true.
    /// If not yet, mark it and return false.
    fn mark_ptr(&mut self, ptr: *mut GCBox<RValue>) -> bool {
        let ptr = ptr as *const GCBox<RValue> as usize;
        let page_ptr = ptr & !(ALIGN - 1);
        let page_info = self
            .pages
            .iter_mut()
            .find(|pinfo| pinfo.ptr.as_ptr() == page_ptr as *mut GCBox<RValue>)
            .unwrap_or_else(|| {
                panic!(
                    "The ptr is not in heap pages. {:?}",
                    page_ptr as *mut GCBox<RValue>
                )
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

    fn sweep_obj(&self, ptr: *mut GCBox<RValue>) -> bool {
        unsafe {
            match (*ptr).inner.kind {
                ObjKind::Array(_) => return false,
                _ => {}
            }
            //eprintln!("free {:?}", (*ptr).inner);
            (*ptr).next = self.free;
            (*ptr).inner.free();
            (*ptr).inner = RValue::new_invalid();
        }
        true
    }

    pub fn sweep(&mut self) {
        let mut c = 0;
        let mut free = self.free;
        loop {
            match free {
                Some(f) => {
                    if self.mark_ptr(f.as_ptr()) {
                        panic!("Marked object in free list.")
                    };
                    free = f.next;
                    c += 1;
                }
                None => break,
            };
        }

        #[cfg(debug_assertions)]
        {
            eprintln!("free list: {}", c);
        }
        c = 0;

        let pinfo = self.pages.last().unwrap();
        let mut ptr = pinfo.ptr.as_ptr();
        assert!(self.used <= PAGE_LEN);
        let i = self.used / 64;
        let bit = self.used % 64;
        for (_j, map) in pinfo.bitmap.iter().take(i).enumerate() {
            let mut map = *map;
            for _b in 0..64 {
                #[cfg(debug_assertions)]
                assert_eq!(
                    ptr as usize - pinfo.ptr.as_ptr() as usize,
                    (_j * 64 + _b) * 64
                );
                if map & 1 == 0 && self.sweep_obj(ptr) {
                    self.free = Some(GCBoxRef::from_ptr(ptr));
                    c += 1;
                }
                ptr = unsafe { ptr.add(1) };
                map >>= 1;
            }
        }

        if i < 64 {
            let mut map = pinfo.bitmap[i];
            for _ in 0..bit {
                if map & 1 == 0 && self.sweep_obj(ptr) {
                    self.free = Some(GCBoxRef::from_ptr(ptr));
                    c += 1;
                }
                ptr = unsafe { ptr.add(1) };
                map >>= 1;
            }
        }

        for pinfo in self.pages[0..self.pages.len() - 1].iter() {
            let mut ptr = pinfo.ptr.as_ptr();
            for (_j, map) in pinfo.bitmap.iter().enumerate() {
                let mut map = *map;
                for _b in 0..64 {
                    #[cfg(debug_assertions)]
                    assert_eq!(
                        ptr as usize - pinfo.ptr.as_ptr() as usize,
                        (_j * 64 + _b) * 64
                    );
                    if map & 1 == 0 && self.sweep_obj(ptr) {
                        self.free = Some(GCBoxRef::from_ptr(ptr));
                        c += 1;
                    }
                    ptr = unsafe { ptr.add(1) };
                    map >>= 1;
                }
            }
        }
        self.sweeped += c;
    }

    // For debug
    #[allow(dead_code)]
    fn check_ptr(&self, ptr: *mut GCBox<RValue>) {
        let ptr = ptr as *const GCBox<RValue> as usize;
        let page_ptr = ptr & !(ALIGN - 1);
        self.pages
            .iter()
            .find(|pinfo| pinfo.ptr.as_ptr() == page_ptr as *mut GCBox<RValue>)
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
