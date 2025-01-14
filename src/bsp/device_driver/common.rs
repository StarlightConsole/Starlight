use core::{fmt, marker::PhantomData, ops};

use crate::memory::{Address, Virtual};

pub struct MMIODerefWrapper<T> {
    start_addr: Address<Virtual>,
    phantom: PhantomData<fn() -> T>
}

#[derive(Copy, Clone)]
pub struct BoundedUsize<const MAX_INCLUSIVE: usize>(usize);

impl<T> MMIODerefWrapper<T> {
    pub const unsafe fn new(start_addr: Address<Virtual>) -> Self {
        Self {
            start_addr,
            phantom: PhantomData
        }
    }
}

impl<T> ops::Deref for MMIODerefWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            &*(self.start_addr.as_usize() as *const _)
        }
    }
}

impl<const MAX_INCLUSIVE: usize> BoundedUsize<{ MAX_INCLUSIVE }> {
    pub const MAX_INCLUSIVE: usize = MAX_INCLUSIVE;

    pub const fn new(number: usize) -> Self {
        assert!(number <= MAX_INCLUSIVE);

        Self(number)
    }

    pub const fn get(self) -> usize {
        self.0
    }
}

impl<const MAX_INCLUSIVE: usize> fmt::Display for BoundedUsize<{ MAX_INCLUSIVE }> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
