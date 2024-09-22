mod gicc;
mod gicd;

use crate::{
    bsp::{self, device_driver::common::BoundedUsize},
    cpu, driver, exception, synchronization,
    synchronization::InitStateLock
};

type HandlerTable = [Option<exception::asynchronous::IRQHandlerDescriptor<IRQNumber>>; IRQNumber::MAX_INCLUSIVE + 1];

pub type IRQNumber = BoundedUsize<{ GICv2::MAX_IRQ_NUMBER }>;

pub struct GICv2 {
    // distributor
    gicd: gicd::GICD,
    
    // cpu interface
    gicc: gicc::GICC,

    // registered IRQ handlers, writeable during kernel init
    handler_table: InitStateLock<HandlerTable>,
}

impl GICv2 {
    const MAX_IRQ_NUMBER: usize = 300; // normally 1019, but keep it lower to save some space

    pub const COMPATIBLE: &'static str = "GICv2 (ARM Generic Interrupt Controller v2)";

    /// # safety
    /// - the user must ensure to provide correct MMIO start addresses
    pub const unsafe fn new(gicd_mmio_start_addr: Address<Virtual>, gicc_mmio_start_addr: Address<Virtual>) -> Self {
        Self {
            gicd: gicd::GICD::new(gicd_mmio_start_addr),
            gicc: gicc::GICC::new(gicc_mmio_start_addr),
            handler_table: InitStateLock::new([None; IRQNumber::MAX_INCLUSIVE + 1]),
        }
    }
}

use synchronization::interface::ReadWriteEx;

impl driver::interface::DeviceDriver for GICv2 {
    type IRQNumberType = IRQNumber;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    unsafe fn init(&self) -> Result<(), &'static str> {
        if bsp::cpu::BOOT_CORE_ID == cpu::smp::core_id() {
            self.gicd.boot_core_init();
        }

        self.gicc.priority_accept_all();
        self.gicc.enable();

        Ok(())
    }
}

impl exception::asynchronous::interface::IRQManager for GICv2 {
    type IRQNumberType = IRQNumber;

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

    fn enable(&self, irq_number: &Self::IRQNumberType) {
        self.gicd.enable(irq_number);
    }

    fn handle_pending_irqs<'irq_context>(&'irq_context self, ic: &exception::asynchronous::IRQContext<'irq_context>) {
        let irq_number = self.gicc.pending_irq_number(ic);

        if irq_number > GICv2::MAX_IRQ_NUMBER {
            return;
        }

        self.handler_table.read(|table| {
            match table[irq_number] {
                None => panic!("No handler registered for IRQ {}", irq_number),
                Some(descriptor) => {
                    descriptor.handler().handle().expect("error handling IRQ");
                }
            }
        });

        self.gicc.mark_completed(irq_number as u32, ic);
    }

    fn print_handler(&self) {
        use crate::info;

        info!("    Peripheral handler:");
        self.handler_table.read(|table| {
            for (i, opt) in table.iter().skip(32).enumerate() {
                if let Some(handler) = opt {
                    info!("        {: >3}. {}", i + 32, handler.name());
                }
            }
        })
    }
}
