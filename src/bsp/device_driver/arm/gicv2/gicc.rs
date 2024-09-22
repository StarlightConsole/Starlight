use aarch64_cpu::registers::{Readable, Writeable};
use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};

use crate::{bsp::device_driver::common::MMIODerefWrapper, exception, memory::{Address, Virtual}};

register_bitfields! {
    u32,

    /// CPU Interface Control Register
    CTLR [
        Enable OFFSET(0) NUMBITS(1) []
    ],

    /// Interrupt Priority Mask Register
    PMR [
        Priority OFFSET(0) NUMBITS(8) []
    ],

    /// Interrupt Acknowledge Register
    IAR [
        InterruptID OFFSET(0) NUMBITS(10) []
    ],

    /// End Of Interrupt Register
    EOIR [
        EOIINTID OFFSET(0) NUMBITS(10) []
    ],
}

register_structs! {
    #[allow(non_snake_case)]
    pub RegisterBlock {
        (0x000 => CTLR: ReadWrite<u32, CTLR::Register>),
        (0x004 => PMR: ReadWrite<u32, PMR::Register>),
        (0x008 => _reserved1),
        (0x00C => IAR: ReadWrite<u32, IAR::Register>),
        (0x010 => EOIR: ReadWrite<u32, EOIR::Register>),
        (0x014 => @END),
    }
}

type Registers = MMIODerefWrapper<RegisterBlock>;

pub struct GICC {
    registers: Registers,
}

impl GICC {
    /// # safety
    /// - the user must ensure to provide a correct MMIO start address
    pub const unsafe fn new(mmio_start_addr: Address<Virtual>) -> Self {
        Self {
            registers: Registers::new(mmio_start_addr),
        }
    }

    pub fn priority_accept_all(&self) {
        self.registers.PMR.write(PMR::Priority.val(255));
    }

    pub fn enable(&self) {
        self.registers.CTLR.write(CTLR::Enable::SET);
    }

    pub fn pending_irq_number<'irq_context>(&self, _ic: &exception::asynchronous::IRQContext<'irq_context>) -> usize {
        self.registers.IAR.read(IAR::InterruptID) as usize
    }

    pub fn mark_completed<'irq_context>(&self, irq_number: u32, _ic: &exception::asynchronous::IRQContext<'irq_context>) {
        self.registers.EOIR.write(EOIR::EOIINTID.val(irq_number));
    }
}
