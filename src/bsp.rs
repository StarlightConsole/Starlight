#[cfg(any(feature = "bsp_rpi3", feature = "bsp_rpi4"))]
mod rpi;

#[cfg(any(feature = "bsp_rpi3", feature = "bsp_rpi4"))]
pub use rpi::*;

mod device_driver;
