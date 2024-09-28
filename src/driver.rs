use alloc::vec::Vec;

use crate::{
    bsp::exception, info, synchronization::{InitStateLock, interface::ReadWriteEx}
};
use core::fmt;

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

pub struct DeviceDriverDescriptor<T> where T: 'static {
    device_driver: &'static (dyn interface::DeviceDriver<IRQNumberType = T> + Sync),
    post_init_callback: Option<DeviceDriverPostInitCallback>,
    irq_number: Option<T>
}

pub struct DriverManager<T> where T: 'static {
    descriptors: InitStateLock<Vec<DeviceDriverDescriptor<T>>>
}

static DRIVER_MANAGER: DriverManager<exception::asynchronous::IRQNumber> = DriverManager::new();

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

impl<T> DriverManager<T> where T: fmt::Display {
    pub const fn new() -> Self {
        Self {
            descriptors: InitStateLock::new(Vec::new())
        }
    }

    pub fn register_driver(&self, descriptor: DeviceDriverDescriptor<T>) {
        self.descriptors.write(|descriptors| descriptors.push(descriptor));
    }

    pub unsafe fn init_drivers_and_irqs(&self) {
        self.descriptors.read(|descriptors| {
            for descriptor in descriptors {
                if let Err(x) = descriptor.device_driver.init() {
                    panic!("Error initializing driver: {}: {}", descriptor.device_driver.compatible(), x);
                }

                if let Some(callback) = &descriptor.post_init_callback {
                    if let Err(x) = callback() {
                        panic!("Error during driver post-init callback: {}: {}", descriptor.device_driver.compatible(), x);
                    }
                }
            }

            for descriptor in descriptors {
                if let Some(irq_number) = &descriptor.irq_number {
                    if let Err(x) = descriptor.device_driver.register_and_enable_irq_handler(irq_number) {
                        panic!("Error during driver interrupt handler registration: {}: {}", descriptor.device_driver.compatible(), x);
                    }
                }
            }
        });
    }

    pub fn enumerate(&self) {
        self.descriptors.read(|descriptors| {
            for (i, desc) in descriptors.iter().enumerate() {
                info!("    {}. {}", i + 1, desc.device_driver.compatible());
            }
        })
    }
}
