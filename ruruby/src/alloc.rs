use crate::RValue;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct RurubyAlloc;

unsafe impl GlobalAlloc for RurubyAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        MALLOC_AMOUNT.fetch_add(layout.size(), Ordering::SeqCst);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        MALLOC_AMOUNT.fetch_sub(layout.size(), Ordering::SeqCst);
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
pub static GLOBAL_ALLOC: RurubyAlloc = RurubyAlloc;

pub static MALLOC_AMOUNT: AtomicUsize = AtomicUsize::new(0);

thread_local!(
    pub static ALLOC: RefCell<Allocator<RValue>> = RefCell::new(Allocator::new());
);

const SIZE: usize = 64;
const GCBOX_SIZE: usize = std::mem::size_of::<RValue>();
const PAGE_LEN: usize = 64 * SIZE;
const DATA_LEN: usize = 64 * (SIZE - 1);
const THRESHOLD: usize = 64 * (SIZE - 2);
const ALLOC_SIZE: usize = PAGE_LEN * GCBOX_SIZE; // 2^18 = 256kb
const MALLOC_THRESHOLD: usize = 256 * 1024;

pub trait GC<T: GCBox> {
    fn mark(&self, alloc: &mut Allocator<T>);
}

pub trait GCRoot<T: GCBox>: GC<T> {
    fn startup_flag(&self) -> bool;
}

pub trait GCBox: PartialEq {
    fn free(&mut self);

    fn next(&self) -> Option<std::ptr::NonNull<Self>>;

    fn set_next_none(&mut self);

    fn set_next(&mut self, next: *mut Self);

    fn new_invalid() -> Self;
}

pub struct Allocator<T> {
    /// Allocated number of objects in current page.
    used_in_current: usize,
    /// Total allocated objects.
    allocated: usize,
    /// Total blocks in free list.
    free_list_count: usize,
    /// Current page.
    current: PageRef<T>,
    /// Info for allocated pages.
    pages: Vec<PageRef<T>>,
    /// Counter of marked objects,
    mark_counter: usize,
    /// List of free objects.
    free: Option<std::ptr::NonNull<T>>,
    /// Deallocated pages.
    free_pages: Vec<PageRef<T>>,
    /// Counter of GC execution.
    count: usize,
    /// Flag for GC timing.
    alloc_flag: bool,
    /// Flag whether GC is enabled or not.
    pub gc_enabled: bool,
    pub malloc_threshold: usize,
}

impl<T: GCBox> Allocator<T> {
    pub fn new() -> Self {
        assert_eq!(64, GCBOX_SIZE);
        assert!(std::mem::size_of::<Page<T>>() <= ALLOC_SIZE);
        let ptr = PageRef::alloc_page();
        Allocator {
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
            malloc_threshold: MALLOC_THRESHOLD,
        }
    }

    #[cfg(not(feature = "gc-stress"))]
    #[inline(always)]
    pub fn is_allocated(&self) -> bool {
        self.alloc_flag
    }

    ///
    /// Returns a number of objects in the free list.
    /// (sweeped objects in the previous GC cycle.)
    ///
    pub fn free_count(&self) -> usize {
        self.free_list_count
    }

    ///
    /// Returns a number of live objects in the previous GC cycle.
    ///
    pub fn live_count(&self) -> usize {
        self.mark_counter
    }

    ///
    /// Returns a number of total allocated objects.
    ///
    pub fn total_allocated(&self) -> usize {
        self.allocated
    }

    ///
    /// Returns a total count of GC execution.
    ///
    pub fn count(&self) -> usize {
        self.count
    }

    ///
    /// Returns total active pages.
    ///
    pub fn pages_len(&self) -> usize {
        self.pages.len() + 1
    }

    ///
    /// Allocate object.
    ///
    pub fn alloc(&mut self, data: T) -> *mut T {
        self.allocated += 1;

        if let Some(gcbox) = self.free {
            // Allocate from the free list.
            let gcbox = gcbox.as_ptr();
            unsafe {
                self.free = (*gcbox).next();
                std::ptr::write(gcbox, data)
            }
            self.free_list_count -= 1;
            return gcbox;
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
            if self.used_in_current == THRESHOLD {
                self.alloc_flag = true;
            }
            let ptr = self.current.get_data_ptr(self.used_in_current);
            self.used_in_current += 1;
            ptr
        };
        #[cfg(feature = "gc-debug")]
        {
            assert!(self.used_in_current <= DATA_LEN);
        }

        unsafe { std::ptr::write(gcbox, data) }
        gcbox
    }

    pub fn gc_mark_only(&mut self, root: &impl GC<T>) {
        self.clear_mark();
        root.mark(self);
        self.print_mark();
    }

    #[inline(always)]
    pub fn check_gc(&mut self, root: &impl GCRoot<T>) {
        let malloced = MALLOC_AMOUNT.load(std::sync::atomic::Ordering::SeqCst);
        #[cfg(not(feature = "gc-stress"))]
        {
            if !self.is_allocated() && !(self.malloc_threshold < malloced) {
                return;
            }
        }
        #[cfg(feature = "gc-debug")]
        dbg!(malloced);
        self.gc(root);
    }

    pub fn gc(&mut self, root: &impl GCRoot<T>) {
        if !self.gc_enabled {
            return;
        }
        #[cfg(feature = "gc-debug")]
        if root.startup_flag() {
            eprintln!("#### GC start");
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
        if root.startup_flag() {
            eprintln!("marked: {}  ", self.mark_counter);
        }
        self.dealloc_empty_pages();
        self.sweep();
        #[cfg(feature = "gc-debug")]
        if root.startup_flag() {
            assert_eq!(self.free_list_count, self.check_free_list());
            eprintln!("free list: {}", self.free_list_count);
        }
        self.alloc_flag = false;
        self.count += 1;
        let malloced = MALLOC_AMOUNT.load(std::sync::atomic::Ordering::SeqCst);
        self.malloc_threshold = malloced + MALLOC_THRESHOLD;
        #[cfg(any(feature = "trace", feature = "gc-debug"))]
        if root.startup_flag() {
            eprintln!("#### GC End");
        }
    }

    /// Mark object.
    /// If object is already marked, return true.
    /// If not yet, mark it and return false.
    pub fn gc_check_and_mark(&mut self, ptr: &T) -> bool {
        let ptr = ptr as *const T as *mut T;
        #[cfg(feature = "gc-debug")]
        self.check_ptr(ptr);
        let mut page_ptr = PageRef::from_inner(ptr);

        let index = unsafe { ptr.offset_from(page_ptr.get_data_ptr(0)) } as usize;
        assert!(index < DATA_LEN);
        let bit_mask = 1 << (index % 64);
        let bitmap = &mut page_ptr.mark_bits_mut()[index / 64];

        let is_marked = (*bitmap & bit_mask) != 0;
        *bitmap |= bit_mask;
        if !is_marked {
            self.mark_counter += 1;
        }
        is_marked
    }
}

impl<T: GCBox> Allocator<T> {
    /// Clear all mark bitmaps.
    fn clear_mark(&mut self) {
        self.current.clear_bits();
        self.pages.iter().for_each(|heap| heap.clear_bits());
        self.mark_counter = 0;
    }

    fn dealloc_empty_pages(&mut self) {
        let len = self.pages.len();
        for i in 0..len {
            if self.pages[len - i - 1].all_dead() {
                let page = self.pages.remove(len - i - 1);
                page.free_page();
                self.free_pages.push(page);
                #[cfg(feature = "gc-debug")]
                eprintln!("dealloc: {:?}", page.0);
            }
        }
    }

    fn sweep_bits(bit: usize, mut map: u64, ptr: &mut *mut T, head: &mut *mut T) -> usize {
        let mut c = 0;
        let min = map.trailing_ones() as usize;
        *ptr = unsafe { (*ptr).add(min) };
        map = map.checked_shr(min as u32).unwrap_or(0);
        for _ in min..bit {
            if map & 1 == 0 {
                unsafe {
                    (**head).set_next(*ptr);
                    *head = *ptr;
                    (**ptr).free();
                    (**ptr).set_next_none();
                    c += 1;
                }
            }
            *ptr = unsafe { (*ptr).add(1) };
            map >>= 1;
        }
        c
    }

    fn sweep(&mut self) {
        let mut c = 0;
        let mut anchor = T::new_invalid();
        let head = &mut ((&mut anchor) as *mut T);

        for pinfo in self.pages.iter() {
            let mut ptr = pinfo.get_data_ptr(0);
            for map in pinfo.mark_bits().iter() {
                c += Allocator::sweep_bits(64, *map, &mut ptr, head);
            }
        }

        let mut ptr = self.current.get_data_ptr(0);
        assert!(self.used_in_current <= DATA_LEN);
        let i = self.used_in_current / 64;
        let bit = self.used_in_current % 64;
        let bitmap = self.current.mark_bits();

        for map in bitmap.iter().take(i) {
            c += Allocator::sweep_bits(64, *map, &mut ptr, head);
        }

        if i < SIZE - 1 {
            c += Allocator::sweep_bits(bit, bitmap[i], &mut ptr, head);
        }

        self.free = anchor.next();
        self.free_list_count = c;
    }
}

// For debug
impl<T: GCBox> Allocator<T> {
    fn check_ptr(&self, ptr: *mut T) {
        let page_ptr = PageRef::from_inner(ptr);
        match self.pages.iter().find(|heap| **heap == page_ptr) {
            Some(_) => return,
            None => {}
        };
        if self.current == page_ptr {
            return;
        };
        eprintln!("dump heap pages");
        self.pages.iter().for_each(|x| eprintln!("{:?}", x.0));
        eprintln!("{:?}", self.current.0);
        unreachable!("The ptr is not in heap pages. {:?}", ptr);
    }

    fn check_free_list(&self) -> usize {
        let mut c = 0;
        let mut free = self.free;
        loop {
            match free {
                Some(f) => {
                    let p = f.as_ptr();
                    self.check_ptr(p);
                    free = unsafe { (*p).next() };
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
                eprintln!();
            }
            i += 1;
        });
    }

    pub(crate) fn print_mark(&self) {
        self.pages.iter().for_each(|pinfo| {
            self.print_bits(pinfo.mark_bits());
            eprintln!("\n");
        });
        self.print_bits(self.current.mark_bits());
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

///
/// Heap page struct.
///
/// Single page occupies `ALLOC_SIZE` bytes in memory.
/// This struct contains 64 * (`SIZE` - 1) `GCBox` cells, and bitmap (`SIZE` - 1 bytes each) for marking phase.
///
struct Page<T> {
    data: [T; DATA_LEN],
    mark_bits: [u64; SIZE - 1],
}

impl<T: GCBox> std::fmt::Debug for Page<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Page")
    }
}

#[derive(PartialEq)]
struct PageRef<T>(*mut Page<T>);

impl<T> Clone for PageRef<T> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T> Copy for PageRef<T> {}

impl<T: GCBox> PageRef<T> {
    ///
    /// Get a reference of the mark bit array.
    ///
    fn mark_bits(&self) -> &[u64; SIZE - 1] {
        unsafe { &(*self.0).mark_bits }
    }

    ///
    /// Get a mutable reference of the mark bit array.
    ///
    fn mark_bits_mut(&mut self) -> &mut [u64; SIZE - 1] {
        unsafe { &mut (*self.0).mark_bits }
    }

    ///
    /// Allocate heap page with `ALLOC_SIZE`.
    ///
    fn alloc_page() -> Self {
        let layout = Layout::from_size_align(ALLOC_SIZE, ALLOC_SIZE).unwrap();
        let ptr = unsafe { System.alloc(layout) };
        #[cfg(feature = "gc-debug")]
        assert_eq!(0, ptr as *const u8 as usize & (ALLOC_SIZE - 1));

        PageRef(ptr as *mut Page<T>)
    }

    /*
    fn dealloc_page(&self) {
        use std::alloc::{dealloc, Layout};
        let layout = Layout::from_size_align(ALLOC_SIZE, ALLOC_SIZE).unwrap();
        unsafe { dealloc(self.as_ptr() as *mut u8, layout) };
    }
    */

    ///
    /// Free all objects in the heap page.
    ///
    fn free_page(&self) {
        let mut ptr = self.get_data_ptr(0);
        for _ in 0..DATA_LEN {
            unsafe { (*ptr).free() };
            ptr = unsafe { ptr.add(1) };
        }
    }

    ///
    /// Get heap page from a RValue pointer.
    ///
    fn from_inner(ptr: *mut T) -> Self {
        PageRef((ptr as usize & !(ALLOC_SIZE - 1)) as *mut Page<T>)
    }

    ///
    /// Get raw pointer of RValue with `index`.
    ///
    fn get_data_ptr(&self, index: usize) -> *mut T {
        unsafe { &(*self.0).data[index] as *const _ as *mut _ }
    }

    ///
    /// Get raw pointer for marking bitmap.
    ///
    fn get_bitmap_ptr(&self) -> *mut [u64; SIZE - 1] {
        unsafe { &(*self.0).mark_bits as *const _ as *mut _ }
    }

    ///
    /// Clear marking bitmap.
    ///
    fn clear_bits(&self) {
        unsafe { std::ptr::write_bytes(self.get_bitmap_ptr(), 0, 1) }
    }

    ///
    /// Check whether all objects were dead.
    ///
    fn all_dead(&self) -> bool {
        unsafe { (*self.0).mark_bits.iter().all(|bits| *bits == 0) }
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
            50.times {
                a = []
                50.times.each {|x|
                    a << Vec.new(x, x)
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
