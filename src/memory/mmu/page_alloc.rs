use core::num::NonZeroUsize;

use crate::{memory::{AddressType, Virtual}, synchronization::IRQSafeNullLock, warn};

use super::MemoryRegion;

pub struct PageAllocator<ATYPE: AddressType> {
    pool: Option<MemoryRegion<ATYPE>>,
}

static KERNEL_MMIO_VA_ALLOCATOR: IRQSafeNullLock<PageAllocator<Virtual>> = IRQSafeNullLock::new(PageAllocator::new());

pub fn kernel_mmio_va_allocator() -> &'static IRQSafeNullLock<PageAllocator<Virtual>> {
    &KERNEL_MMIO_VA_ALLOCATOR
}

impl<ATYPE: AddressType> PageAllocator<ATYPE> {
    pub const fn new() -> Self {
        Self {
            pool: None,
        }
    }

    #[allow(unused)]
    pub fn init(&mut self, pool: MemoryRegion<ATYPE>) {
        if self.pool.is_some() {
            warn!("already initialized");
            return;
        }

        self.pool = Some(pool);
    }

    pub fn alloc(&mut self, num_requested_pages: NonZeroUsize) -> Result<MemoryRegion<ATYPE>, &'static str> {
        if self.pool.is_none() {
            return Err("allocator not initialized");
        }

        self.pool.as_mut().unwrap().take_first_n_pages(num_requested_pages)
    }
}
