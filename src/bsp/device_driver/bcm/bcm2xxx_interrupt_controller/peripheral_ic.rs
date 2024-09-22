use super::{PendingIRQs, PeripheralIRQ};
use crate::{
    bsp::device_driver::common::MMIODerefWrapper, exception, memory::{Address, Virtual}, synchronization::{interface::{Mutex, ReadWriteEx}, IRQSafeNullLock, InitStateLock}
};

use tock_registers::{
    interfaces::{Readable, Writeable},
    register_structs,
    registers::{ReadOnly, WriteOnly},
};

register_structs! {
    #[allow(non_snake_case)]
    WORegisterBlock {
        (0x00 => _reserved1),
        (0x10 => ENABLE_1: WriteOnly<u32>),
        (0x14 => ENABLE_2: WriteOnly<u32>),
        (0x18 => @END),
    }
}

register_structs! {
    #[allow(non_snake_case)]
    RORegisterBlock {
        (0x00 => _reserved1),
        (0x04 => PENDING_1: ReadOnly<u32>),
        (0x08 => PENDING_2: ReadOnly<u32>),
        (0x0c => @END),
    }
}

type WriteOnlyRegisters = MMIODerefWrapper<WORegisterBlock>;
type ReadOnlyRegisters = MMIODerefWrapper<RORegisterBlock>;

type HandlerTable = [Option<exception::asynchronous::IRQHandlerDescriptor<PeripheralIRQ>>; PeripheralIRQ::MAX_INCLUSIVE + 1];

pub struct PeripheralIC {
    wo_registers: IRQSafeNullLock<WriteOnlyRegisters>,
    ro_registers: ReadOnlyRegisters,
    handler_table: InitStateLock<HandlerTable>,
}

impl PeripheralIC {
    /// # safety
    /// - the user must ensure to provide a correct MMIO start address
    pub const unsafe fn new(mmio_start_addr: Address<Virtual>) -> Self {
        Self {
            wo_registers: IRQSafeNullLock::new(WriteOnlyRegisters::new(mmio_start_addr)),
            ro_registers: ReadOnlyRegisters::new(mmio_start_addr),
            handler_table: InitStateLock::new([None; PeripheralIRQ::MAX_INCLUSIVE + 1]),
        }
    }

    fn pending_irqs(&self) -> PendingIRQs {
        let pending_mask: u64 = (u64::from(self.ro_registers.PENDING_2.get()) << 32)
            | u64::from(self.ro_registers.PENDING_1.get());

        PendingIRQs::new(pending_mask)
    }
}

impl exception::asynchronous::interface::IRQManager for PeripheralIC {
    type IRQNumberType = PeripheralIRQ;

    fn register_handler(&self, irq_handler_descriptor: exception::asynchronous::IRQHandlerDescriptor<Self::IRQNumberType>) -> Result<(), &'static str> {
        self.handler_table.write(|table| {
            let irq_number = irq_handler_descriptor.number().get();

            if table[irq_number].is_some() {
                return Err("IRQ handler already registered");
            }

            table[irq_number] = Some(irq_handler_descriptor);

            Ok(())
        })
    }

    fn enable(&self, irq: &Self::IRQNumberType) {
        self.wo_registers.lock(|regs| {
            let enable_reg = match irq.get() {
                0..=31 => &regs.ENABLE_1,
                _ => &regs.ENABLE_2,
            };

            let enable_bit: u32 = 1 << (irq.get() % 32);

            enable_reg.set(enable_bit);
        });
    }

    fn handle_pending_irqs<'irq_context>(&'irq_context self, _ic: &exception::asynchronous::IRQContext<'irq_context>) {
        self.handler_table.read(|table| {
            for irq_number in self.pending_irqs() {
                match table[irq_number] {
                    None => panic!("No handler registered for IRQ {}", irq_number),
                    Some(descriptor) => {
                        descriptor.handler().handle().expect("error handling IRQ");
                    },
                }
            }
        })
    }

    fn print_handler(&self) {
        use crate::info;

        info!("    Peripheral handler:");

        self.handler_table.read(|table| {
            for (i, opt) in table.iter().enumerate() {
                if let Some(handler) = opt {
                    info!("        {: >3}. {}", i, handler.name());
                }
            }
        });
    }
}
