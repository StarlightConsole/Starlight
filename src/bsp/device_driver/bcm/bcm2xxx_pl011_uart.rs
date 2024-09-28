use crate::{
    bsp::device_driver::common::MMIODerefWrapper, console, cpu, driver, exception::{self, asynchronous::IRQNumber}, memory::{Address, Virtual}, synchronization::{interface::Mutex, IRQSafeNullLock}
};

use core::fmt;
use tock_registers::{
    interfaces::{Readable, Writeable},
    register_bitfields, register_structs, registers::{ReadOnly, ReadWrite, WriteOnly}
};

// TODO: add missing comments for the register blocks
register_bitfields! {
    u32,
    FR [
        TXFE OFFSET(7) NUMBITS(1) [],
        TXFF OFFSET(5) NUMBITS(1) [],
        RXFE OFFSET(4) NUMBITS(1) [],
        BUSY OFFSET(3) NUMBITS(1) []
    ],

    IBRD [
        BAUD_DIVINT OFFSET(0) NUMBITS(16) []
    ],
    FBRD [
        BAUD_DIVFRAC OFFSET(0) NUMBITS(6) []
    ],

    LCR_H [
        #[allow(clippy::enum_variant_names)]
        WLEN OFFSET(5) NUMBITS(2) [
            FiveBit = 0b00,
            SixBit = 0b01,
            SevenBit = 0b10,
            EightBit = 0b11
        ],
        FEN OFFSET(4) NUMBITS(1) [
            FifosDisabled = 0b0,
            FifosEnabled = 0b1
        ]
    ],

    CR [
        RXE OFFSET(9) NUMBITS(1) [
            Disabled = 0b0,
            Enabled = 0b1
        ],
        TXE OFFSET(8) NUMBITS(1) [
            Disabled = 0b0,
            Enabled = 0b1
        ],

        UARTEN OFFSET(0) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1
        ]
    ],

    // Interrupt FIFO Level Select Register
    IFLS [
        RXIFLSEL OFFSET(3) NUMBITS(5) [
            OneEight = 0b000,
            OneQuarter = 0b001,
            OneHalf = 0b010,
            ThreeQuarters = 0b011,
            SevenEights = 0b100
        ]
    ],

    // Interrupt Mask Set/Clear Register
    IMSC [
        RTIM OFFSET(6) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1
        ],

        RXIM OFFSET(4) NUMBITS(1) [
            Disabled = 0,
            Enabled = 1
        ]
    ],

    // Masked Interrut Status Register
    MIS [
        RTMIS OFFSET(6) NUMBITS(1) [],
        RXMIS OFFSET(4) NUMBITS(1) []
    ],

    // Interrupt Clear Register
    ICR [
        ALL OFFSET(0) NUMBITS(11) []
    ]
}

register_structs! {
    #[allow(non_snake_case)]
    pub RegisterBlock {
        (0x00 => DR: ReadWrite<u32>),
        (0x04 => _reserved1),
        (0x18 => FR: ReadOnly<u32, FR::Register>),
        (0x1c => _reserved2),
        (0x24 => IBRD: WriteOnly<u32, IBRD::Register>),
        (0x28 => FBRD: WriteOnly<u32, FBRD::Register>),
        (0x2c => LCR_H: WriteOnly<u32, LCR_H::Register>),
        (0x30 => CR: WriteOnly<u32, CR::Register>),
        (0x34 => IFLS: ReadWrite<u32, IFLS::Register>),
        (0x38 => IMSC: ReadWrite<u32, IMSC::Register>),
        (0x3C => _reserved3),
        (0x40 => MIS: ReadOnly<u32, MIS::Register>),
        (0x44 => ICR: WriteOnly<u32, ICR::Register>),
        (0x48 => @END),
    }
}

type Registers = MMIODerefWrapper<RegisterBlock>;

#[derive(PartialEq)]
enum BlockingMode {
    Blocking,
    NonBlocking
}

struct PL011UartInner {
    registers: Registers,
    chars_written: usize,
    chars_read: usize
}

pub struct PL011Uart {
    inner: IRQSafeNullLock<PL011UartInner>
}

impl PL011UartInner {
    pub const unsafe fn new(mmio_start_addr: Address<Virtual>) -> Self {
        Self {
            registers: Registers::new(mmio_start_addr),
            chars_written: 0,
            chars_read: 0
        }
    }

    pub fn init(&mut self) {
        self.flush();

        self.registers.CR.set(0);
        self.registers.ICR.write(ICR::ALL::CLEAR);

        // BRD Calculation (The clock is set to 48 MHz in config.txt)
        // (48_000_000 / 16) / 921_600 = 3.2552083
        // Integer part (IBRD) = 3, Fractional part = 0.2552083
        // FBRD = INTEGER((0.2552083 * 64) + 0.5) = 16
        self.registers.IBRD.write(IBRD::BAUD_DIVINT.val(3));
        self.registers.FBRD.write(FBRD::BAUD_DIVFRAC.val(16));

        self.registers.LCR_H.write(LCR_H::WLEN::EightBit + LCR_H::FEN::FifosEnabled);

        self.registers.IFLS.write(IFLS::RXIFLSEL::OneEight);
        self.registers.IMSC.write(IMSC::RXIM::Enabled + IMSC::RTIM::Enabled);

        self.registers.CR.write(CR::UARTEN::Enabled + CR::TXE::Enabled + CR::RXE::Enabled);
    }

    fn write_char(&mut self, c: char) {
        while self.registers.FR.matches_all(FR::TXFF::SET) {
            cpu::nop();
        }

        self.registers.DR.set(c as u32);
        self.chars_written += 1;
    }

    fn write_array(&mut self, a: &[char]) {
        for c in a {
            self.write_char(*c);
        }
    }

    fn flush(&self) {
        while self.registers.FR.matches_all(FR::BUSY::SET) {
            cpu::nop();
        }
    }

    fn read_char_converting(&mut self, blocking_mode: BlockingMode) -> Option<char> {
        if self.registers.FR.matches_all(FR::RXFE::SET) {
            if blocking_mode == BlockingMode::NonBlocking {
                return None;
            }

            while self.registers.FR.matches_all(FR::RXFE::SET) {
                cpu::nop();
            }
        }

        let mut ret = self.registers.DR.get() as u8 as char;

        if ret == '\r' {
            ret = '\n';
        }

        self.chars_read += 1;

        Some(ret)
    }
}

impl fmt::Write for PL011UartInner {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }

        Ok(())
    }
}

impl PL011Uart {
    pub const COMPATIBLE: &'static str = "BCM PL011 UART";

    pub const unsafe fn new(mmio_start_addr: Address<Virtual>) -> Self {
        Self {
            inner: IRQSafeNullLock::new(PL011UartInner::new(mmio_start_addr))
        }
    }
}

impl driver::interface::DeviceDriver for PL011Uart {
    type IRQNumberType = IRQNumber;

    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    unsafe fn init(&self) -> Result<(), &'static str> {
        self.inner.lock(|inner| inner.init());

        Ok(())
    }

    fn register_and_enable_irq_handler(&'static self, irq_number: &Self::IRQNumberType) -> Result<(), &'static str> {
        use exception::asynchronous::{irq_manager, IRQHandlerDescriptor};

        let descriptor = IRQHandlerDescriptor::new(*irq_number, Self::COMPATIBLE, self);

        irq_manager().register_handler(descriptor)?;
        irq_manager().enable(irq_number);

        Ok(())
    }
}

impl console::interface::Write for PL011Uart {
    fn write_char(&self, c: char) {
        self.inner.lock(|inner| inner.write_char(c));
    }

    fn write_array(&self, a: &[char]) {
        self.inner.lock(|inner| inner.write_array(a));
    }

    fn write_fmt(&self, args: core::fmt::Arguments) -> fmt::Result {
        self.inner.lock(|inner| fmt::Write::write_fmt(inner, args))
    }

    fn flush(&self) {
        self.inner.lock(|inner| inner.flush());
    }
}

impl console::interface::Read for PL011Uart {
    fn read_char(&self) -> char {
        self.inner.lock(|inner| inner.read_char_converting(BlockingMode::Blocking).expect("when using BlockingMode::Blocking, a character should always be read"))
    }

    fn clear_rx(&self) {
        while self.inner.lock(|inner| inner.read_char_converting(BlockingMode::NonBlocking)).is_some() {}
    }
}

impl console::interface::Statistics for PL011Uart {
    fn chars_written(&self) -> usize {
        self.inner.lock(|inner| inner.chars_written)
    }

    fn chars_read(&self) -> usize {
        self.inner.lock(|inner| inner.chars_read)
    }
}

impl console::interface::All for PL011Uart {}

impl exception::asynchronous::interface::IRQHandler for PL011Uart {
    fn handle(&self) -> Result<(), &'static str> {
        self.inner.lock(|inner| {
            let pending = inner.registers.MIS.extract();
            
            inner.registers.ICR.write(ICR::ALL::CLEAR);

            if pending.matches_any(MIS::RXMIS::SET + MIS::RTMIS::SET) {
                // echo any received chars
                while let Some(c) = inner.read_char_converting(BlockingMode::NonBlocking) {
                    inner.write_char(c)
                }
            }
        });

        Ok(())
    }
}
