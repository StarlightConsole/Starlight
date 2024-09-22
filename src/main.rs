#![allow(incomplete_features)]
#![allow(internal_features)]

#![feature(asm_const)]
#![feature(const_option)]
#![feature(format_args_nl)]
#![feature(trait_alias)]
#![feature(generic_const_exprs)]
#![feature(step_trait)]
#![feature(core_intrinsics)]

#![no_main]
#![no_std]

//! The `kernel` binary.
//!
//! # Starlight Kernel
//! The `kernel` binary is the entry point for the Starlight Operating System.
//!
//! Starlight is a game console operating system for the handheld Starlight gaming device.

mod bsp;
mod comet;
mod common;
mod console;
mod cpu;
mod driver;
mod exception;
mod memory;
mod panic_wait;
mod print;
mod state;
mod synchronization;
mod time;

#[no_mangle]
unsafe fn kernel_init() -> ! {
    exception::handling_init();

    memory::init();

    if let Err(x) = bsp::driver::init() {
        panic!("error initializing BSP driver subsystem: {}", x);
    }

    driver::driver_manager().init_drivers_and_irqs();

    bsp::memory::mmu::kernel_add_mapping_records_for_precomputed();

    exception::asynchronous::local_irq_unmask();
    
    state::state_manager().transition_to_single_core_main();

    // leave the unsafe world
    kernel_main();
}

fn kernel_main() -> ! {
    comet::set_device(comet::Device::Starlight);

    info!("{} version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("booting on: {}", bsp::board_name());

    info!("enabled MMU, mappings:");
    memory::mmu::kernel_print_mappings();

    let (_, privilege_level) = exception::current_privilege_level();
    info!("current privilege level: {}", privilege_level);

    info!("exception handling state:");
    exception::asynchronous::print_state();

    info!("architectural timer resolution: {} ns", time::time_manager().resolution().as_nanos());

    info!("drivers loaded:");
    driver::driver_manager().enumerate();

    info!("registered IRQ handlers:");
    exception::asynchronous::irq_manager().print_handler();

    info!("echoing input now");
    cpu::wait_forever();
}
