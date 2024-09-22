#[cfg(target_arch = "aarch64")]
#[path = "arch/aarch64/exception.rs"]
mod arch_exception;

pub mod asynchronous;

pub use arch_exception::*;

#[allow(missing_docs)]
#[derive(Eq, PartialEq)]
pub enum PrivilegeLevel {
    User,
    Kernel,
    Hypervisor,
    Unknown,
}
