#[cfg(target_arch = "aarch64")]
#[path = "../../arch/aarch64/memory/mmu/translation_table.rs"]
mod arch_translation_table;

#[allow(unused)]
pub use arch_translation_table::*;

use crate::memory::{mmu::{AttributeFields, MemoryRegion}, Address, Physical, Virtual};

pub mod interface {

    use super::*;

    pub trait TranslationTable {
        /// # safety
        /// - implementor must ensure that this function can run only once or is harmless if
        ///   invoked multiple times
        fn init(&mut self);

        fn phys_base_address(&self) -> Address<Physical>;

        /// # safety
        /// - using wrong attributes can cause multiple issues of different nature in the system
        /// - it is not required that the architectural implementation prevents aliasing, that is,
        ///   mapping to the same physical memory using multiple virtual addresses, which would break
        ///   Rust's ownership assumptions. this should be protected against in the kernel's generic
        ///   MMU code
        unsafe fn map_at(&mut self, virt_region: &MemoryRegion<Virtual>, phys_region: &MemoryRegion<Physical>, attr: &AttributeFields) -> Result<(), &'static str>;
    }
}
