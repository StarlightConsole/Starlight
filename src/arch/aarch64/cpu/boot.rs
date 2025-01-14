use core::arch::global_asm;

use aarch64_cpu::{asm, registers::*};

use crate::memory::{self, Address};

global_asm!(
    include_str!("boot.s"),
    CONST_CURRENTEL_EL2 = const 0x8,
    CONST_CORE_ID_MASK = const 0b11
);

/// # safety
/// - the `bss` section is not initialized yet, the code can't use or reference it in any way
/// - the hw state of EL1 must be prepared in a sound way
#[inline(always)]
unsafe fn prepare_el2_to_el1_transition(virt_boot_core_stack_end_exclusive_addr: u64, virt_kernel_init_addr: u64) {
    // enable timer counter registers for EL1
    CNTHCTL_EL2.write(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);

    // no offset for reading the counters
    CNTVOFF_EL2.set(0);

    // set EL1 execution state to AArch64
    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

    // set up a simulated exception return

    // fake a saved program status where all interrupts were masked and SP_EL1 was used as a stack
    // pointer
    SPSR_EL2.write(
        SPSR_EL2::D::Masked
            + SPSR_EL2::A::Masked
            + SPSR_EL2::I::Masked
            + SPSR_EL2::F::Masked
            + SPSR_EL2::M::EL1h
    );

    // let the link register point to kernel_init
    ELR_EL2.set(virt_kernel_init_addr);

    // set up SP_EL1 (stack pointer), which will be used by EL1 once we "return" to it. since there
    // are no plans to return to EL2, just re-use the same stack
    SP_EL1.set(virt_boot_core_stack_end_exclusive_addr);
}

/// the rust entry of the `kernel` binary
/// called from the assembly `_start` function
///
/// # safety
/// - exception return from EL2 must continue execution in EL1 with `kernel_init()`
#[no_mangle]
pub unsafe extern "C" fn _start_rust(phys_kernel_tables_base_addr: u64, virt_boot_core_stack_end_exclusive_addr: u64, virt_kernel_init_addr: u64) -> ! {
    prepare_el2_to_el1_transition(virt_boot_core_stack_end_exclusive_addr, virt_kernel_init_addr);

    // turn on the MMU for EL1
    let addr = Address::new(phys_kernel_tables_base_addr as usize);
    memory::mmu::enable_mmu_and_caching(addr).unwrap();

    // use `eret` to "return" to EL1, this results in execution of kernel_init() in EL1
    asm::eret()
}
