use core::fmt;

use crate::{bsp::device_driver::common::BoundedUsize, driver, exception::{self, asynchronous::IRQHandlerDescriptor}, memory::{Address, Virtual}};

mod peripheral_ic;

struct PendingIRQs {
    bitmask: u64,
}

pub type LocalIRQ = BoundedUsize<{ InterruptController::MAX_LOCAL_IRQ_NUMBER }>;
pub type PeripheralIRQ = BoundedUsize<{ InterruptController::MAX_PERIPHERAL_IRQ_NUMBER }>;

#[derive(Copy, Clone)]
#[allow(unused)]
pub enum IRQNumber {
    Local(LocalIRQ),
    Peripheral(PeripheralIRQ),
}

pub struct InterruptController {
    peripheral_ic: peripheral_ic::PeripheralIC,
}

impl PendingIRQs {
    pub fn new(bitmask: u64) -> Self {
        Self { bitmask }
    }
}

impl Iterator for PendingIRQs {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bitmask == 0 {
            return None;
        }

        let next = self.bitmask.trailing_zeros() as usize;
        self.bitmask &= self.bitmask.wrapping_sub(1);
        Some(next)
    }
}

impl fmt::Display for IRQNumber {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Local(number) => write!(f, "Local({})", number),
            Self::Peripheral(number) => write!(f, "Peripheral({})", number),
        }
    }
}

impl InterruptController {
    const MAX_LOCAL_IRQ_NUMBER: usize = 3;
    const MAX_PERIPHERAL_IRQ_NUMBER: usize = 63;

    pub const COMPATIBLE: &'static str = "BCM Interrupt Controller";

    /// # safety
    /// - the user must ensure to provide a correct MMIO start address
    pub const unsafe fn new(peripheral_ic_mmio_start_addr: Address<Virtual>) -> Self {
        Self {
            peripheral_ic: peripheral_ic::PeripheralIC::new(peripheral_ic_mmio_start_addr),
        }
    }
}

impl driver::interface::DeviceDriver for InterruptController {
    type IRQNumberType = IRQNumber;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }
}

impl exception::asynchronous::interface::IRQManager for InterruptController {
    type IRQNumberType = IRQNumber;

    fn register_handler(&self, irq_handler_descriptor: exception::asynchronous::IRQHandlerDescriptor<Self::IRQNumberType>) -> Result<(), &'static str> {
        match irq_handler_descriptor.number() {
            IRQNumber::Local(_) => unimplemented!("local IRQ controller not implemented"),
            IRQNumber::Peripheral(pirq) => {
                let peripheral_descriptor = IRQHandlerDescriptor::new(
                    pirq,
                    irq_handler_descriptor.name(),
                    irq_handler_descriptor.handler(),
                );

                self.peripheral_ic.register_handler(peripheral_descriptor)
            }
        }
    }

    fn enable(&self, irq: &Self::IRQNumberType) {
        match irq {
            IRQNumber::Local(_) => unimplemented!("local IRQ controller not implemented"),
            IRQNumber::Peripheral(pirq) => self.peripheral_ic.enable(pirq),
        }
    }

    fn handle_pending_irqs<'irq_context>(&'irq_context self, ic: &exception::asynchronous::IRQContext<'irq_context>) {
        // can only be a peripheral IRQ cause enable() doesn't support local IRQs
        self.peripheral_ic.handle_pending_irqs(ic)
    }

    fn print_handler(&self) {
        self.peripheral_ic.print_handler();
    }
}
