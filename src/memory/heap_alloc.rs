use core::{alloc::{GlobalAlloc, Layout}, sync::atomic::{AtomicBool, Ordering}};

use crate::{bsp, common, debug, info, memory::{Address, Virtual}, synchronization::{interface::Mutex, IRQSafeNullLock}, warn};

use linked_list_allocator::Heap as LinkedListHeap;

pub struct HeapAllocator {
    inner: IRQSafeNullLock<LinkedListHeap>,
}

#[global_allocator]
static KERNEL_HEAP_ALLOCATOR: HeapAllocator = HeapAllocator::new();

#[inline(always)]
fn debug_print_alloc_dealloc(operation: &'static str, ptr: *mut u8, layout: Layout) {
    let size = layout.size();
    let (size_h, size_unit) = common::size_human_readable_ceil(size);
    let addr = Address::<Virtual>::new(ptr as usize);

    debug!("kernel heap: {}", operation);
    debug!("    size:     {:#x} ({} {})", size, size_h, size_unit);
    debug!("    start:    {}", addr);
    debug!("    end excl: {}", addr + size);
    debug!("");
    // TODO: backtrace::Backtrace
    debug!("    (backtrace functionality not yet implemented)");
}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}

pub fn kernel_heap_allocator() -> &'static HeapAllocator {
    &KERNEL_HEAP_ALLOCATOR
}

impl HeapAllocator {
    pub const fn new() -> Self {
        Self {
            inner: IRQSafeNullLock::new(LinkedListHeap::empty()),
        }
    }

    pub fn print_usage(&self) {
        let (used, free) = KERNEL_HEAP_ALLOCATOR.inner.lock(|inner| (inner.used(), inner.free()));

        if used >= 1024 {
            let (used_h, used_unit) = common::size_human_readable_ceil(used);
            info!("    used: {} bytes ({} {})", used, used_h, used_unit);
        } else {
            info!("    used: {} bytes", used);
        }

        if free >= 1024 {
            let (free_h, free_unit) = common::size_human_readable_ceil(free);
            info!("    free: {} bytes ({} {})", free, free_h, free_unit);
        } else {
            info!("    free: {} bytes", free);
        }
    }
}

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let result = KERNEL_HEAP_ALLOCATOR.inner.lock(|inner| inner.allocate_first_fit(layout).ok());

        match result {
            None => core::ptr::null_mut(),
            Some(allocation) => {
                let ptr = allocation.as_ptr();
                debug_print_alloc_dealloc("allocation", ptr, layout);
                ptr
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        KERNEL_HEAP_ALLOCATOR.inner.lock(|inner| inner.deallocate(core::ptr::NonNull::new_unchecked(ptr), layout));

        debug_print_alloc_dealloc("free", ptr, layout);
    }
}

pub fn kernel_init_heap_allocator() {
    static INIT_DONE: AtomicBool = AtomicBool::new(false);
    if INIT_DONE.load(Ordering::Relaxed) {
        warn!("kernel heap allocator already initialized!");
        return;
    }

    let region = bsp::memory::mmu::virt_heap_region();

    KERNEL_HEAP_ALLOCATOR.inner.lock(|inner| unsafe {
        inner.init(region.start_addr().as_usize() as *mut u8, region.size());
    });

    INIT_DONE.store(true, Ordering::Relaxed);
}
