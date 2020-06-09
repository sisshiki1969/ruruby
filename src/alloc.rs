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
const DATA_LEN: usize = 64 * 63;
const ALIGN: usize = 0x4_0000; // 2^18 = 256kb
const ALLOC_SIZE: usize = PAGE_LEN * GCBOX_SIZE;

pub trait GC {
    fn mark(&self, alloc: &mut Allocator);
}

///
/// Heap page struct.
///
/// Single page ocupies 256kb in memory.
/// This struct contains 64 * 63 GCBox cells, and bitmap (8 * 64 bytes each) for marking phase.
///
struct Page {
    data: [GCBox<RValue>; DATA_LEN],
    mark_bits: [u64; 63],
}

type PageRef = Ref<Page>;

impl PageRef {
    ///
    /// Allocate heap page with `ALLOC_SIZE` and `ALIGN`.
    ///
    fn alloc_page() -> Self {
        use std::alloc::{alloc, Layout};
        let layout = Layout::from_size_align(ALLOC_SIZE, ALIGN).unwrap();
        let ptr = unsafe { alloc(layout) };
        #[cfg(debug_assertions)]
        {
            assert_eq!(0, ptr as *const u8 as usize & (ALIGN - 1));
            //eprintln!("page allocated: {:?}", ptr);
        }
        PageRef::from_ptr(ptr as *mut Page)
    }

    fn dealloc_page(&self) {
        use std::alloc::{dealloc, Layout};
        let layout = Layout::from_size_align(ALLOC_SIZE, ALIGN).unwrap();
        unsafe { dealloc(self.as_ptr() as *mut u8, layout) };
    }

    ///
    /// Get raw pointer for inner GCBox with `index`.
    ///
    fn get_data_ptr(&self, index: usize) -> *mut GCBox<RValue> {
        &self.data[index] as *const GCBox<RValue> as *mut GCBox<RValue>
    }

    ///
    /// Get raw pointer for marking bitmap.
    ///
    fn get_bitmap_ptr(&self) -> *mut [u64; 63] {
        &self.mark_bits as *const [u64; 63] as *mut [u64; 63]
    }

    ///
    /// Clear marking bitmap.
    ///
    fn clear_bits(&self) {
        unsafe { std::ptr::write_bytes(self.get_bitmap_ptr(), 0, 1) }
    }
}

///
/// Container for "GC-able" objects.
///
/// This struct contains inner object data and a pointer to the next GCBox in free list.
///
#[derive(Debug, Clone)]
pub struct GCBox<T: GC> {
    inner: T,
    next: Option<GCBoxRef<T>>,
}

impl GCBox<RValue> {
    fn new() -> Self {
        GCBox {
            inner: RValue::new_invalid(),
            next: None,
        }
    }

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
    /// Total blocks in free list.
    free_list_count: usize,
    /// Current page.
    current: PageRef,
    /// Info for allocated pages.
    pages: Vec<PageRef>,
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

impl Allocator {
    pub fn new() -> Self {
        assert_eq!(56, std::mem::size_of::<RValue>());
        assert_eq!(64, GCBOX_SIZE);
        assert!(std::mem::size_of::<Page>() <= ALLOC_SIZE);
        let ptr = PageRef::alloc_page();
        Allocator {
            used: 0,
            allocated: 0,
            free_list_count: 0,
            current: ptr,
            pages: vec![],
            mark_counter: 0,
            free: None,
        }
    }

    pub fn free_count(&self) -> usize {
        self.free_list_count
    }

    /// Allocate object.
    pub fn alloc(&mut self, data: RValue) -> *mut GCBox<RValue> {
        self.allocated += 1;
        ALLOC_THREAD.with(|m| {
            let mut m = m.borrow_mut();
            m.allocated += 1;
            m.alloc_flag = m.allocated % 2048 == 0;
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
                self.free_list_count -= 1;
                return gcbox.as_ptr();
            }
            None => {}
        }

        let gcbox = if self.used == DATA_LEN {
            // Allocate new page.
            self.used = 1;
            self.pages.push(self.current);
            self.current = PageRef::alloc_page();
            self.current.get_data_ptr(0)
        } else {
            // Bump allocation.
            let ptr = self.current.get_data_ptr(self.used);
            self.used += 1;
            ptr
        };
        #[cfg(debug_assertions)]
        {
            assert!(self.used <= DATA_LEN);
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
            eprintln!("free list: {}", self.free_list_count);
        }
        ALLOC_THREAD.with(|m| {
            m.borrow_mut().alloc_flag = false;
        });
        #[cfg(debug_assertions)]
        {
            self.print_mark();
            eprintln!("--GC completed");
        }
    }

    /// Clear all mark bitmaps.
    fn clear_mark(&mut self) {
        self.current.clear_bits();
        self.pages.iter().for_each(|heap| heap.clear_bits());
        self.mark_counter = 0;
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
        #[cfg(debug_assertions)]
        self.check_ptr(ptr);
        let ptr = ptr as *const GCBox<RValue> as usize;
        let mut page_ptr = PageRef::from_ptr((ptr & !(ALIGN - 1)) as *mut Page);

        let offset = ptr - page_ptr.get_data_ptr(0) as usize;
        let index = offset / GCBOX_SIZE;
        #[cfg(debug_assertions)]
        {
            assert_eq!(0, offset % GCBOX_SIZE);
            assert!(index < DATA_LEN);
        }
        let bit_mask = 1 << (index % 64);
        let bitmap = &mut page_ptr.mark_bits[index / 64];

        let is_marked = (*bitmap & bit_mask) != 0;
        *bitmap |= bit_mask;
        if !is_marked {
            self.mark_counter += 1;
        }
        is_marked
    }

    fn sweep_obj(ptr: *mut GCBox<RValue>, head: &mut *mut GCBox<RValue>) -> bool {
        unsafe {
            match (*ptr).inner.kind {
                ObjKind::Array(_) => return false,
                _ => {}
            }; /*
               println!(
                   "free {:?} {:?}",
                   &(*ptr).inner as *const RValue,
                   (*ptr).inner
               );*/
            (**head).next = Some(GCBoxRef::from_ptr(ptr));
            *head = ptr;
            (*ptr).next = None;
            (*ptr).inner.free();
            (*ptr).inner = RValue::new_invalid();
        }
        true
    }

    fn sweep_bits(
        bit: usize,
        mut map: u64,
        ptr: &mut *mut GCBox<RValue>,
        head: &mut *mut GCBox<RValue>,
    ) -> usize {
        let mut c = 0;
        for _ in 0..bit {
            if map & 1 == 0 && Allocator::sweep_obj(*ptr, head) {
                c += 1;
            }
            *ptr = unsafe { (*ptr).add(1) };
            map >>= 1;
        }
        c
    }

    pub fn sweep(&mut self) {
        let mut c = 0;
        let mut anchor = GCBox::new();
        let head = &mut ((&mut anchor) as *mut GCBox<RValue>);
        let mut ptr = self.current.get_data_ptr(0);
        assert!(self.used <= PAGE_LEN);
        let i = self.used / 64;
        let bit = self.used % 64;
        let bitmap = &self.current.mark_bits;

        for (_j, map) in bitmap.iter().take(i).enumerate() {
            c += Allocator::sweep_bits(64, *map, &mut ptr, head);
        }

        if i < 63 {
            c += Allocator::sweep_bits(bit, bitmap[i], &mut ptr, head);
        }

        for pinfo in &self.pages {
            let mut ptr = pinfo.get_data_ptr(0);
            for map in pinfo.mark_bits.iter() {
                c += Allocator::sweep_bits(64, *map, &mut ptr, head);
            }
        }
        self.free = anchor.next;
        self.free_list_count = c;
    }

    // For debug
    #[allow(dead_code)]
    fn check_ptr(&self, ptr: *mut GCBox<RValue>) {
        let ptr = ptr as *const GCBox<RValue> as usize;
        let page_ptr = (ptr & !(ALIGN - 1)) as *mut Page;
        match self.pages.iter().find(|heap| heap.as_ptr() == page_ptr) {
            Some(_) => return,
            None => {}
        };
        if self.current.as_ptr() == page_ptr {
            return;
        };
        panic!("The ptr is not in heap pages.");
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
    fn print_bits(&self, bitmap: &[u64; 63]) {
        let mut i = 0;
        bitmap.iter().for_each(|m| {
            eprint!("{:016x} ", m.reverse_bits());
            if i % 8 == 7 {
                eprintln!("");
            }
            i += 1;
        });
    }

    #[allow(dead_code)]
    pub fn print_mark(&self) {
        self.pages.iter().for_each(|pinfo| {
            self.print_bits(&pinfo.mark_bits);
            eprintln!("\n");
        });
        self.print_bits(&self.current.mark_bits);
        eprintln!("\n");
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
