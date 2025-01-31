use crate::bsp;

pub use bsp::device_driver::IRQNumber;

#[cfg(feature = "bsp_rpi3")]
pub(in crate::bsp) mod irq_map {
    use super::bsp::device_driver::{IRQNumber, PeripheralIRQ};

    pub const PL011_UART: IRQNumber = IRQNumber::Peripheral(PeripheralIRQ::new(57));
}

#[cfg(feature = "bsp_rpi4")]
pub(in crate::bsp) mod irq_map {
    use super::bsp::device_driver::IRQNumber;

    pub const PL011_UART: IRQNumber = IRQNumber::new(57);
}
