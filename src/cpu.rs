#[cfg(target_arch = "aarch64")]
#[path = "arch/aarch64/cpu.rs"]
mod arch_cpu;
pub use arch_cpu::*;

mod boot;
pub mod smp;
