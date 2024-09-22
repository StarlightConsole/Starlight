use aarch64_cpu::registers::{ID_AA64MMFR0_EL1, MAIR_EL1, SCTLR_EL1, TCR_EL1, TTBR0_EL1};
use tock_registers::interfaces::*;
use crate::memory::mmu::MMUEnableError;

use crate::memory::{Address, Physical};
use crate::{bsp, memory};

use super::TranslationGranule;
use core::intrinsics::unlikely;

use aarch64_cpu::asm::barrier;
use tock_registers::interfaces::Writeable;

struct MemoryManagementUnit;

pub type Granule512MiB = TranslationGranule<{ 512 * 1024 * 1024 }>;
pub type Granule64KiB = TranslationGranule<{ 64 * 1024 }>;

pub mod mair {
    pub const DEVICE: u64 = 0;
    pub const NORMAL: u64 = 1;
}

static MMU: MemoryManagementUnit = MemoryManagementUnit;

impl<const AS_SIZE: usize> crate::memory::mmu::AddressSpace<AS_SIZE> {
    pub const fn arch_address_space_size_sanity_checks() {
        // size must be at least one full 512MiB table
        assert!((AS_SIZE % Granule512MiB::SIZE) == 0);

        // check for 48-bit virtual address size as maximum, which is supported by any ARMv8
        // version.
        assert!(AS_SIZE <= (1 << 48));
    }
}

impl MemoryManagementUnit {
    fn set_up_mair(&self) {
        // define the memory types being mapped
        MAIR_EL1.write(
            // attribute 1 - cacheable normal DRAM
            MAIR_EL1::Attr1_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc +
            MAIR_EL1::Attr1_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc +

            // attribute 0 - device
            MAIR_EL1::Attr0_Device::nonGathering_nonReordering_EarlyWriteAck
        );
    }

    fn configure_translation_control(&self) {
        let t0sz = (64 - bsp::memory::mmu::KernelVirtAddrSpace::SIZE_SHIFT) as u64;

        TCR_EL1.write(
            TCR_EL1::TBI0::Used
                + TCR_EL1::IPS::Bits_40
                + TCR_EL1::TG0::KiB_64
                + TCR_EL1::SH0::Inner
                + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
                + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
                + TCR_EL1::EPD0::EnableTTBR0Walks
                + TCR_EL1::A1::TTBR0
                + TCR_EL1::T0SZ.val(t0sz)
                + TCR_EL1::EPD1::DisableTTBR1Walks
        );
    }
}

pub fn mmu() -> &'static impl crate::memory::mmu::interface::MMU {
    &MMU
}


impl memory::mmu::interface::MMU for MemoryManagementUnit {
    unsafe fn enable_mmu_and_caching(&self, phys_tables_base_addr: Address<Physical>) -> Result<(), MMUEnableError> {
        if unlikely(self.is_enabled()) {
            return Err(MMUEnableError::AlreadyEnabled);
        }

        if unlikely(!ID_AA64MMFR0_EL1.matches_all(ID_AA64MMFR0_EL1::TGran64::Supported)) {
            return Err(MMUEnableError::Other("Translation granule not supported in HW"));
        }

        self.set_up_mair();

        TTBR0_EL1.set_baddr(phys_tables_base_addr.as_usize() as u64);

        self.configure_translation_control();

        // switch the MMU on

        // force all previous changes to be seen before the MMU is enabled
        barrier::isb(barrier::SY);

        // enable the MMU and turn on data and instruction caching
        SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);

        // force MMU init to complete before next instruction
        barrier::isb(barrier::SY);

        Ok(())
    }

    #[inline(always)]
    fn is_enabled(&self) -> bool {
        SCTLR_EL1.matches_all(SCTLR_EL1::M::Enable)
    }
}
