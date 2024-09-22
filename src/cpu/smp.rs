#[cfg(target_arch = "aarch64")]
#[path = "../arch/aarch64/cpu/smp.rs"]
mod arch_smp;

#[allow(unused)]
pub use arch_smp::*;
