use crate::{
    bsp::exception, info, synchronization::{InitStateLock, interface::ReadWriteEx}
};
use core::fmt;

const NUM_DRIVERS: usize = 5;

struct DriverManagerInner<T> where T: 'static {
    next_index: usize,
    descriptors: [Option<DeviceDriverDescriptor<T>>; NUM_DRIVERS]
}

pub mod interface {
    pub trait DeviceDriver {
        type IRQNumberType: super::fmt::Display;

        fn compatible(&self) -> &'static str;

        unsafe fn init(&self) -> Result<(), &'static str> {
            Ok(())
        }

        fn register_and_enable_irq_handler(&'static self, irq_number: &Self::IRQNumberType) -> Result<(), &'static str> {
            panic!("attempt to enable IRQ {} for device {}, but driver does not support this", irq_number, self.compatible())
        }
    }
}

pub type DeviceDriverPostInitCallback = unsafe fn() -> Result<(), &'static str>;

#[derive(Copy, Clone)]
pub struct DeviceDriverDescriptor<T> where T: 'static {
    device_driver: &'static (dyn interface::DeviceDriver<IRQNumberType = T> + Sync),
    post_init_callback: Option<DeviceDriverPostInitCallback>,
    irq_number: Option<T>
}

pub struct DriverManager<T> where T: 'static {
    inner: InitStateLock<DriverManagerInner<T>>
}

static DRIVER_MANAGER: DriverManager<exception::asynchronous::IRQNumber> = DriverManager::new();

impl<T> DriverManagerInner<T> where T: 'static + Copy {
    pub const fn new() -> Self {
        Self {
            next_index: 0,
            descriptors: [None; NUM_DRIVERS]
        }
    }
}

impl<T> DeviceDriverDescriptor<T> {
    pub fn new(device_driver: &'static (dyn interface::DeviceDriver<IRQNumberType = T> + Sync), post_init_callback: Option<DeviceDriverPostInitCallback>, irq_number: Option<T>) -> Self {
        Self {
            device_driver,
            post_init_callback,
            irq_number,
        }
    }
}

pub fn driver_manager() -> &'static DriverManager<exception::asynchronous::IRQNumber> {
    &DRIVER_MANAGER
}

impl<T> DriverManager<T> where T: fmt::Display + Copy {
    pub const fn new() -> Self {
        Self {
            inner: InitStateLock::new(DriverManagerInner::new())
        }
    }

    pub fn register_driver(&self, descriptor: DeviceDriverDescriptor<T>) {
        self.inner.write(|inner| {
            inner.descriptors[inner.next_index] = Some(descriptor);
            inner.next_index += 1;
        });
    }

    fn for_each_descriptor<'a>(&'a self, f: impl FnMut(&'a DeviceDriverDescriptor<T>)) {
        self.inner.read(|inner| {
            inner.descriptors
                .iter()
                .filter_map(|x| x.as_ref())
                .for_each(f)
        });
    }

    pub unsafe fn init_drivers_and_irqs(&self) {
        self.for_each_descriptor(|descriptor| {
            if let Err(x) = descriptor.device_driver.init() {
                panic!("Error initializing driver: {}: {}", descriptor.device_driver.compatible(), x);
            }

            if let Some(callback) = &descriptor.post_init_callback {
                if let Err(x) = callback() {
                    panic!("Error during driver post-init callback: {}: {}", descriptor.device_driver.compatible(), x);
                }
            }
        });

        self.for_each_descriptor(|descriptor| {
            if let Some(irq_number) = &descriptor.irq_number {
                if let Err(x) = descriptor
                    .device_driver
                    .register_and_enable_irq_handler(irq_number)
                {
                    panic!("error during driver interrupt handler registration: {}: {}", descriptor.device_driver.compatible(), x)
                }
            }
        });
    }

    pub fn enumerate(&self) {
        let mut i: usize = 1;

        self.for_each_descriptor(|descriptor| {
            info!("    {}. {}", i, descriptor.device_driver.compatible());
            i += 1;
        });
    }
}
