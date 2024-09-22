use core::cell::UnsafeCell;

use crate::{exception, state};

pub mod interface {
    pub trait Mutex {
        type Data;

        fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R;
    }

    pub trait ReadWriteEx {
        type Data;

        fn write<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R;
        fn read<'a, R>(&'a self, f: impl FnOnce(&'a Self::Data) -> R) -> R;
    }
}

pub struct IRQSafeNullLock<T> where T: ?Sized {
    data: UnsafeCell<T>,
}

pub struct InitStateLock<T> where T: ?Sized {
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for IRQSafeNullLock<T> where T: ?Sized + Send {}
unsafe impl<T> Sync for IRQSafeNullLock<T> where T: ?Sized + Send {}

impl<T> IRQSafeNullLock<T> {
    #[allow(unused)]
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }
}

unsafe impl<T> Send for InitStateLock<T> where T: ?Sized + Send {}
unsafe impl<T> Sync for InitStateLock<T> where T: ?Sized + Send {}

impl<T> InitStateLock<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
        }
    }
}

impl<T> interface::Mutex for IRQSafeNullLock<T> {
    type Data = T;

    fn lock<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R {
        let data = unsafe { &mut *self.data.get() };

        exception::asynchronous::exec_with_irq_masked(|| f(data))
    }
}

impl<T> interface::ReadWriteEx for InitStateLock<T> {
    type Data = T;

    fn write<'a, R>(&'a self, f: impl FnOnce(&'a mut Self::Data) -> R) -> R {
        assert!(state::state_manager().is_init(), "InitStateLock::write called after kernel init phase");
        assert!(!exception::asynchronous::is_local_irq_masked(), "InitStateLock::write called with IRQs unmasked");

        let data = unsafe { &mut *self.data.get() };

        f(data)
    }

    fn read<'a, R>(&'a self, f: impl FnOnce(&'a Self::Data) -> R) -> R {
        let data = unsafe { &*self.data.get() };

        f(data)
    }
}
