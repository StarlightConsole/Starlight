use super::{exception, memory::map::mmio};
use crate::{bsp::device_driver, console, driver as generic_driver, exception as generic_exception, memory::{self, mmu::MMIODescriptor}};
use core::{mem::MaybeUninit, sync::atomic::{AtomicBool, Ordering}};

static mut PL011_UART: MaybeUninit<device_driver::PL011Uart> = MaybeUninit::uninit();
static mut GPIO: MaybeUninit<device_driver::GPIO> = MaybeUninit::uninit();

#[cfg(feature = "bsp_rpi3")]
static mut INTERRUPT_CONTROLLER: MaybeUninit<device_driver::InterruptController> = MaybeUninit::uninit();

#[cfg(feature = "bsp_rpi4")]
static mut INTERRUPT_CONTROLLER: MaybeUninit<device_driver::GICv2> = MaybeUninit::uninit();

unsafe fn instantiate_uart() -> Result<(), &'static str> {
    let mmio_descriptor = MMIODescriptor::new(mmio::PL011_UART_START, mmio::PL011_UART_SIZE);
    let virt_addr = memory::mmu::kernel_map_mmio(device_driver::PL011Uart::COMPATIBLE, &mmio_descriptor)?;

    PL011_UART.write(device_driver::PL011Uart::new(virt_addr));

    Ok(())
}

unsafe fn post_init_uart() -> Result<(), &'static str> {
    console::register_console(PL011_UART.assume_init_ref());
    Ok(())
}

unsafe fn instantiate_gpio() -> Result<(), &'static str> {
    let mmio_descriptor = MMIODescriptor::new(mmio::GPIO_START, mmio::GPIO_SIZE);
    let virt_addr = memory::mmu::kernel_map_mmio(device_driver::GPIO::COMPATIBLE, &mmio_descriptor)?;

    GPIO.write(device_driver::GPIO::new(virt_addr));

    Ok(())
}

unsafe fn post_init_gpio() -> Result<(), &'static str> {
    GPIO.assume_init_ref().map_pl011_uart();
    Ok(())
}

#[cfg(feature = "bsp_rpi3")]
unsafe fn instantiate_interrupt_controller() -> Result<(), &'static str> {
    let mmio_descriptor = MMIODescriptor::new(mmio::PERIPHERAL_IC_START, mmio::PERIPHERAL_IC_SIZE);
    let virt_addr = memory::mmu::kernel_map_mmio(device_driver::InterruptController::COMPATIBLE, &mmio_descriptor)?;

    INTERRUPT_CONTROLLER.write(device_driver::InterruptController::new(virt_addr));

    Ok(())
}

#[cfg(feature = "bsp_rpi4")]
unsafe fn instantiate_interrupt_controller() -> Result<(), &'static str> {
    let gicd_mmio_descriptor = MMIODescriptor::new(mmio::GICD_START, mmio::GICD_SIZE);
    let gicd_virt_addr = memory::mmu::kernel_map_mmio("GICv2 GICD", &gicd_mmio_descriptor)?;

    let gicc_mmio_descriptor = MMIODescriptor::new(mmio::GICC_START, mmio::GICC_SIZE);
    let gicc_virt_addr = memory::mmu::kernel_map_mmio("GICv2 GICC", &gicc_mmio_descriptor)?;

    INTERRUPT_CONTROLLER.write(device_driver::GICv2::new(gicd_virt_addr, gicc_virt_addr));

    Ok(())
}

unsafe fn post_init_interrupt_controller() -> Result<(), &'static str> {
    generic_exception::asynchronous::register_irq_manager(INTERRUPT_CONTROLLER.assume_init_ref());

    Ok(())
}

unsafe fn init_driver_uart() -> Result<(), &'static str> {
    instantiate_uart()?;

    let uart_descriptor = generic_driver::DeviceDriverDescriptor::new(PL011_UART.assume_init_ref(), Some(post_init_uart), Some(exception::asynchronous::irq_map::PL011_UART));
    generic_driver::driver_manager().register_driver(uart_descriptor);

    Ok(())
}

unsafe fn init_driver_gpio() -> Result<(), &'static str> {
    instantiate_gpio()?;

    let gpio_descriptor = generic_driver::DeviceDriverDescriptor::new(GPIO.assume_init_ref(), Some(post_init_gpio), None);
    generic_driver::driver_manager().register_driver(gpio_descriptor);

    Ok(())
}

unsafe fn init_driver_interrupt_controller() -> Result<(), &'static str> {
    instantiate_interrupt_controller()?;

    let interrupt_controller_descriptor = generic_driver::DeviceDriverDescriptor::new(INTERRUPT_CONTROLLER.assume_init_ref(), Some(post_init_interrupt_controller), None);
    generic_driver::driver_manager().register_driver(interrupt_controller_descriptor);

    Ok(())
}

pub unsafe fn init() -> Result<(), &'static str> {
    static INIT_DONE: AtomicBool = AtomicBool::new(false);
    if INIT_DONE.load(Ordering::Relaxed) {
        return Err("Init already done");
    }

    init_driver_uart()?;
    init_driver_gpio()?;
    init_driver_interrupt_controller()?;

    INIT_DONE.store(true, Ordering::Relaxed);
    Ok(())
}
