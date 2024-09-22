#[cfg(target_arch = "aarch64")]
#[path = "../arch/aarch64/exception/asynchronous.rs"]
mod arch_asynchronous;

mod null_irq_manager;

use core::marker::PhantomData;

use crate::{bsp, synchronization};

pub use arch_asynchronous::*;

pub type IRQNumber = bsp::exception::asynchronous::IRQNumber;

#[derive(Copy, Clone)]
pub struct IRQHandlerDescriptor<T> where T: Copy {
    number: T,
    name: &'static str,
    handler: &'static (dyn interface::IRQHandler + Sync)
}

#[derive(Clone, Copy)]
pub struct IRQContext<'irq_context> {
    _0: PhantomData<&'irq_context ()>,
}

pub mod interface {
    pub trait IRQHandler {
        fn handle(&self) -> Result<(), &'static str>;
    }

    pub trait IRQManager {
        type IRQNumberType: Copy;

        fn register_handler(&self, irq_handler_descriptor: super::IRQHandlerDescriptor<Self::IRQNumberType>) -> Result<(), &'static str>;
        fn enable(&self, irq_number: &Self::IRQNumberType);
        fn handle_pending_irqs<'irq_context>(&'irq_context self, ic: &super::IRQContext<'irq_context>);

        fn print_handler(&self) {}
    }
}

static CUR_IRQ_MANAGER: InitStateLock<&'static (dyn interface::IRQManager<IRQNumberType = IRQNumber> + Sync)> = InitStateLock::new(&null_irq_manager::NULL_IRQ_MANAGER);

use synchronization::{interface::ReadWriteEx, InitStateLock};

impl<T> IRQHandlerDescriptor<T> where T: Copy {
    pub const fn new(number: T, name: &'static str, handler: &'static (dyn interface::IRQHandler + Sync)) -> Self {
        Self {
            number,
            name,
            handler,
        }
    }

    pub const fn number(&self) -> T {
        self.number
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }

    pub const fn handler(&self) -> &'static (dyn interface::IRQHandler + Sync) {
        self.handler
    }
}

impl<'irq_context> IRQContext<'irq_context> {
    /// # safety
    /// - this must only be called when the current core is in an interrupt context and will not
    ///   live beyond the end of it. that is, creation is allowed in interrupt vector functions. for
    ///   example, in the ARMv8-A case, in `extern "C" fn current_elx_irq()`.
    /// - note that the lifetime `'irq_context` of the returned instance is unconstrained. user
    ///   code must not be able to influence the lifetime picked for this type, since that might
    ///   cause it to be inferred to `'static`.
    #[inline(always)]
    pub unsafe fn new() -> Self {
        IRQContext {
            _0: PhantomData,
        }
    }
}

#[inline(always)]
pub fn exec_with_irq_masked<T>(f: impl FnOnce() -> T) -> T {
    let saved = local_irq_mask_save();
    let ret = f();
    local_irq_restore(saved);

    ret
}

pub fn register_irq_manager(new_manager: &'static (dyn interface::IRQManager<IRQNumberType = IRQNumber> + Sync)) {
    CUR_IRQ_MANAGER.write(|manager| *manager = new_manager);
}

pub fn irq_manager() -> &'static dyn interface::IRQManager<IRQNumberType = IRQNumber> {
    CUR_IRQ_MANAGER.read(|manager| *manager)
}
