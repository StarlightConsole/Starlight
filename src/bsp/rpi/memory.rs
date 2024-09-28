use core::cell::UnsafeCell;
use crate::memory::{mmu::PageAddress, Address, Physical, Virtual};

pub mod mmu;

extern "Rust" {
    static __code_start: UnsafeCell<()>;
    static __code_end_exclusive: UnsafeCell<()>;

    static __data_start: UnsafeCell<()>;
    static __data_end_exclusive: UnsafeCell<()>;

    static __mmio_remap_start: UnsafeCell<()>;
    static __mmio_remap_end_exclusive: UnsafeCell<()>;

    static __heap_start: UnsafeCell<()>;
    static __heap_end_exclusive: UnsafeCell<()>;

    static __boot_core_stack_start: UnsafeCell<()>;
    static __boot_core_stack_end_exclusive: UnsafeCell<()>;
}

pub(super) mod map {
    use super::*;

    #[cfg(feature = "bsp_rpi3")]
    pub mod mmio {
        use super::*;

        pub const PERIPHERAL_IC_START: Address<Physical> = Address::new(0x3F00_B200);
        pub const PERIPHERAL_IC_SIZE: usize = 0x24;

        pub const GPIO_START: Address<Physical> = Address::new(0x3F20_0000);
        pub const GPIO_SIZE: usize = 0xA0;
        
        pub const PL011_UART_START: Address<Physical> = Address::new(0x3F20_1000);
        pub const PL011_UART_SIZE: usize = 0x48;

        pub const END: Address<Physical> = Address::new(0x4001_0000);
    }

    #[cfg(feature = "bsp_rpi4")]
    pub mod mmio {
        use super::*;

        pub const GPIO_START: Address<Physical> = Address::new(0xFE20_0000);
        pub const GPIO_SIZE: usize = 0xA0;

        pub const PL011_UART_START: Address<Physical> = Address::new(0xFE20_1000);
        pub const PL011_UART_SIZE: usize = 0x48;

        pub const GICD_START: Address<Physical> = Address::new(0xFF84_1000);
        pub const GICD_SIZE: usize = 0x824;

        pub const GICC_START: Address<Physical> = Address::new(0xFF84_2000);
        pub const GICC_SIZE: usize = 0x14;

        pub const END: Address<Physical> = Address::new(0xFF85_0000);
    }

    pub const END: Address<Physical> = mmio::END;
}

/// # safety
/// - value is provided by linker script and must be trusted as-is
#[inline(always)]
fn virt_code_start() -> PageAddress<Virtual> {
    PageAddress::from(unsafe { __code_start.get() as usize })
}

/// # safety
/// - value is provided by linker script and must be trusted as-is
#[inline(always)]
fn code_size() -> usize {
    unsafe { (__code_end_exclusive.get() as usize) - (__code_start.get() as usize) }
}

/// # safety
/// - value is provided by linker script and must be trusted as-is
#[inline(always)]
fn virt_data_start() -> PageAddress<Virtual> {
    PageAddress::from(unsafe { __data_start.get() as usize })
}

/// # safety
/// - value is provided by linker script and must be trusted as-is
fn data_size() -> usize {
    unsafe { (__data_end_exclusive.get() as usize) - (__data_start.get() as usize) }
}

/// # safety
/// - value is provided by linker script and must be trusted as-is
#[inline(always)]
fn virt_mmio_remap_start() -> PageAddress<Virtual> {
    PageAddress::from(unsafe { __mmio_remap_start.get() as usize })
}

/// # safety
/// - value is provided by linker script and must be trusted as-is
fn mmio_remap_size() -> usize {
    unsafe { (__mmio_remap_end_exclusive.get() as usize) - (__mmio_remap_start.get() as usize) }
}

/// # safety
/// - value is provided by linker script and must be trusted as-is
#[inline(always)]
fn virt_heap_start() -> PageAddress<Virtual> {
    PageAddress::from(unsafe { __heap_start.get() as usize })
}

/// # safety
/// - value is provided by linker script and must be trusted as-is
fn heap_size() -> usize {
    unsafe { (__heap_end_exclusive.get() as usize) - (__heap_start.get() as usize) }
}

/// # safety
/// - value is provided by linker script and must be trusted as-is
#[inline(always)]
fn virt_boot_core_stack_start() -> PageAddress<Virtual> {
    PageAddress::from(unsafe { __boot_core_stack_start.get() as usize })
}

/// # safety
/// - value is provided by linker script and must be trusted as-is
fn boot_core_stack_size() -> usize {
    unsafe { (__boot_core_stack_end_exclusive.get() as usize) - (__boot_core_stack_start.get() as usize) }
}

#[inline(always)]
pub fn phys_addr_space_end_exclusive_addr() -> PageAddress<Physical> {
    PageAddress::from(map::END)
}
