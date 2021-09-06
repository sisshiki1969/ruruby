use crate::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

struct RurubyAllocator;

unsafe impl GlobalAlloc for RurubyAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        MALLOC_AMOUNT.fetch_add(layout.size(), Ordering::Relaxed);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        MALLOC_AMOUNT.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static GLOBAL: RurubyAllocator = RurubyAllocator;

pub static MALLOC_AMOUNT: AtomicUsize = AtomicUsize::new(0);

thread_local!(
    pub static ALLOC: RefCell<Allocator> = RefCell::new(Allocator::new());
);

const SIZE: usize = 64;
const GCBOX_SIZE: usize = std::mem::size_of::<GCBox<RValue>>();
const PAGE_LEN: usize = 64 * SIZE;
const DATA_LEN: usize = 64 * (SIZE - 1);
const THRESHOLD: usize = 64 * (SIZE - 2);
const ALLOC_SIZE: usize = PAGE_LEN * GCBOX_SIZE; // 2^18 = 256kb

pub trait GC {
    fn mark(&self, alloc: &mut Allocator);
}

///-----------------------------------------------------------------------------------------------------------------
///
/// Heap page struct.
///
/// Single page occupies `ALLOC_SIZE` bytes in memory.
/// This struct contains 64 * (`SIZE` - 1) `GCBox` cells, and bitmap (`SIZE` - 1 bytes each) for marking phase.
///
///-----------------------------------------------------------------------------------------------------------------
struct Page {
    data: [GCBox<RValue>; DATA_LEN],
    mark_bits: [u64; SIZE - 1],
}

impl std::fmt::Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Page")
    }
}

type PageRef = Ref<Page>;

impl PageRef {
    ///
    /// Allocate heap page with `ALLOC_SIZE` and `ALIGN`.
    ///
    fn alloc_page() -> Self {
        //use std::alloc::{alloc, Layout};
        let layout = Layout::from_size_align(ALLOC_SIZE, ALLOC_SIZE).unwrap();
        let ptr = unsafe { System.alloc(layout) };
        #[cfg(feature = "gc-debug")]
        assert_eq!(0, ptr as *const u8 as usize & (ALLOC_SIZE - 1));

        PageRef::from_ptr(ptr as *mut Page)
    }

    /*
    fn dealloc_page(&self) {
        use std::alloc::{dealloc, Layout};
        let layout = Layout::from_size_align(ALLOC_SIZE, ALLOC_SIZE).unwrap();
        unsafe { dealloc(self.as_ptr() as *mut u8, layout) };
    }
    */

    fn free_page(&self) {
        let mut ptr = self.get_data_ptr(0);
        for _ in 0..DATA_LEN {
            unsafe { (*ptr).free() };
            ptr = unsafe { ptr.add(1) };
        }
    }

    fn from_inner(ptr: *mut GCBox<RValue>) -> Self {
        PageRef::from_ptr((ptr as usize & !(ALLOC_SIZE - 1)) as *mut Page)
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
    fn get_bitmap_ptr(&self) -> *mut [u64; SIZE - 1] {
        &self.mark_bits as *const [u64; SIZE - 1] as *mut [u64; SIZE - 1]
    }

    ///
    /// Clear marking bitmap.
    ///
    fn clear_bits(&self) {
        unsafe { std::ptr::write_bytes(self.get_bitmap_ptr(), 0, 1) }
    }

    fn all_dead(&self) -> bool {
        self.mark_bits.iter().all(|bits| *bits == 0)
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

    pub fn inner(&self) -> &RValue {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut RValue {
        &mut self.inner
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

#[derive(Debug)]
pub struct Allocator {
    /// Allocated number of objects in current page.
    used_in_current: usize,
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
    /// Deallocated pages.
    free_pages: Vec<PageRef>,
    /// Counter of GC execution.
    count: usize,
    /// Flag for GC timing.
    alloc_flag: bool,
    /// Flag whether GC is enabled or not.
    pub gc_enabled: bool,
    pub malloc_threshold: usize,
}

impl Allocator {
    pub fn new() -> Self {
        assert_eq!(56, std::mem::size_of::<RValue>());
        assert_eq!(64, GCBOX_SIZE);
        assert!(std::mem::size_of::<Page>() <= ALLOC_SIZE);
        let ptr = PageRef::alloc_page();
        let alloc = Allocator {
            used_in_current: 0,
            allocated: 0,
            free_list_count: 0,
            current: ptr,
            pages: vec![],
            mark_counter: 0,
            free: None,
            free_pages: vec![],
            count: 0,
            alloc_flag: false,
            gc_enabled: true,
            malloc_threshold: 1000000,
        };
        alloc
    }

    pub fn is_allocated(&self) -> bool {
        self.alloc_flag
    }

    /// Returns number of objects in the free list.
    /// (sweeped objects in the previous GC.)
    pub fn free_count(&self) -> usize {
        self.free_list_count
    }

    /// Returns number of live objects in the previous GC.
    pub fn live_count(&self) -> usize {
        self.mark_counter
    }

    /// Returns number of total allocated objects.
    pub fn total_allocated(&self) -> usize {
        self.allocated
    }

    /// Return total count of GC execution.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Return total active pages.
    pub fn pages_len(&self) -> usize {
        self.pages.len() + 1
    }

    /// Allocate object.
    pub fn alloc(&mut self, data: RValue) -> *mut GCBox<RValue> {
        self.allocated += 1;

        if let Some(gcbox) = self.free {
            // Allocate from the free list.
            self.free = gcbox.next;
            #[cfg(feature = "gc-debug")]
            assert!(gcbox.inner.is_invalid());
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

        let gcbox = if self.used_in_current == DATA_LEN {
            // Allocate new page.
            self.used_in_current = 1;
            self.pages.push(self.current);
            self.current = self
                .free_pages
                .pop()
                .unwrap_or_else(|| PageRef::alloc_page());
            self.current.get_data_ptr(0)
        } else {
            // Bump allocation.
            if self.used_in_current == THRESHOLD && self.gc_enabled {
                self.alloc_flag = true;
            }
            let ptr = self.current.get_data_ptr(self.used_in_current);
            self.used_in_current += 1;
            ptr
        };
        #[cfg(feature = "gc-debug")]
        {
            assert!(self.used_in_current <= DATA_LEN);
            assert!(0 < self.used_in_current);
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

    pub fn gc_mark_only(&mut self, root: &Globals) {
        self.clear_mark();
        root.mark(self);
        self.print_mark();
    }

    pub fn gc(&mut self, root: &Globals) {
        #[cfg(any(feature = "trace", feature = "gc-debug"))]
        {
            eprintln!("#### GC Start");
        }
        #[cfg(feature = "gc-debug")]
        {
            eprintln!(
                "allocated: {}  used in current page: {}  allocated pages: {}",
                self.allocated,
                self.used_in_current,
                self.pages.len()
            );
        }
        self.clear_mark();
        root.mark(self);
        #[cfg(feature = "gc-debug")]
        eprintln!("marked: {}  ", self.mark_counter);
        self.dealloc_empty_pages();
        self.sweep();
        #[cfg(feature = "gc-debug")]
        {
            assert_eq!(self.free_list_count, self.check_free_list());
            eprintln!("free list: {}", self.free_list_count);
        }
        self.alloc_flag = false;
        self.count += 1;
        #[cfg(any(feature = "trace", feature = "gc-debug"))]
        {
            eprintln!("#### GC End");
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
        #[cfg(feature = "gc-debug")]
        self.check_ptr(ptr);
        let mut page_ptr = PageRef::from_inner(ptr);

        let offset = ptr as usize - page_ptr.get_data_ptr(0) as usize;
        let index = offset / GCBOX_SIZE;
        #[cfg(feature = "gc-debug")]
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

    pub fn dealloc_empty_pages(&mut self) {
        let len = self.pages.len();
        for i in 0..len {
            if self.pages[len - i - 1].all_dead() {
                let page = self.pages.remove(len - i - 1);
                page.free_page();
                self.free_pages.push(page);
                #[cfg(feature = "gc-debug")]
                eprintln!("dealloc: {:?}", page.as_ptr());
            }
        }
    }

    fn sweep_bits(
        bit: usize,
        mut map: u64,
        ptr: &mut *mut GCBox<RValue>,
        head: &mut *mut GCBox<RValue>,
    ) -> usize {
        let mut c = 0;
        let min = map.trailing_ones() as usize;
        *ptr = unsafe { (*ptr).add(min) };
        map = map.checked_shr(min as u32).unwrap_or(0);
        for _ in min..bit {
            if map & 1 == 0 {
                unsafe {
                    (**head).next = Some(GCBoxRef::from_ptr(*ptr));
                    *head = *ptr;
                    (**ptr).next = None;
                    (**ptr).inner.free();
                    c += 1;
                }
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

        for pinfo in self.pages.iter() {
            let mut ptr = pinfo.get_data_ptr(0);
            for map in pinfo.mark_bits.iter() {
                c += Allocator::sweep_bits(64, *map, &mut ptr, head);
            }
        }

        let mut ptr = self.current.get_data_ptr(0);
        assert!(self.used_in_current <= DATA_LEN);
        let i = self.used_in_current / 64;
        let bit = self.used_in_current % 64;
        let bitmap = &self.current.mark_bits;

        for map in bitmap.iter().take(i) {
            c += Allocator::sweep_bits(64, *map, &mut ptr, head);
        }

        if i < SIZE - 1 {
            c += Allocator::sweep_bits(bit, bitmap[i], &mut ptr, head);
        }

        self.free = anchor.next;
        self.free_list_count = c;
    }
}

// For debug
impl Allocator {
    fn check_ptr(&self, ptr: *mut GCBox<RValue>) {
        let page_ptr = PageRef::from_inner(ptr);
        match self
            .pages
            .iter()
            .find(|heap| heap.as_ptr() == page_ptr.as_ptr())
        {
            Some(_) => return,
            None => {}
        };
        if self.current.as_ptr() == page_ptr.as_ptr() {
            return;
        };
        eprintln!("dump heap pages");
        self.pages
            .iter()
            .for_each(|x| eprintln!("{:?}", x.as_ptr()));
        eprintln!("{:?}", self.current.as_ptr());
        unreachable!("The ptr is not in heap pages. {:?}", ptr);
    }

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

    fn print_bits(&self, bitmap: &[u64; SIZE - 1]) {
        let mut i = 0;
        bitmap.iter().for_each(|m| {
            eprint!("{:016x} ", m.reverse_bits());
            if i % 8 == 7 {
                eprintln!("");
            }
            i += 1;
        });
    }

    pub fn print_mark(&self) {
        self.pages.iter().for_each(|pinfo| {
            self.print_bits(&pinfo.mark_bits);
            eprintln!("\n");
        });
        self.print_bits(&self.current.mark_bits);
        eprintln!("\n");
        eprintln!(
            "GC Info----------------------------------------------------------------------------"
        );
        eprintln!(
            "active pages: {} free pages:{}",
            self.pages.len() + 1,
            self.free_pages.len(),
        );
        assert_eq!(self.free_list_count, self.check_free_list());
        eprintln!(
            "free list:{} allocated:{}  used in current page:{}",
            self.free_list_count, self.allocated, self.used_in_current
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

    #[test]
    fn gc_test() {
        let program = r#"
            class Vec
                def initialize(x,y)
                    @x = x
                    @y = y
                end
            end
            300.times {
                a = []
                50.times.each {|x|
                    a << Vec.new(x.to_s, x.to_s)
                }
                b = {}
                50.times.each {|x|
                    b[x.to_s] = [x...(x*2), (x+1).to_s, (x+2).to_s]
                }
                c = Fiber.new {}
            }
        "#;
        assert_script(program);
    }
}
