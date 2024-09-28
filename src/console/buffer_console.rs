use core::fmt::{self, Write};

use crate::{
    console, debug,
    synchronization::{interface::ReadWriteEx, InitStateLock},
    warn,
};

use super::interface;

const BUF_SIZE: usize = 64 * 1024;

pub struct BufferConsoleInner {
    buf: [char; BUF_SIZE],
    write_ptr: usize,
}

pub struct BufferConsole {
    inner: InitStateLock<BufferConsoleInner>,
}

pub static BUFFER_CONSOLE: BufferConsole = BufferConsole {
    inner: InitStateLock::new(BufferConsoleInner {
        buf: ['\0'; BUF_SIZE],
        write_ptr: 0,
    }),
};

impl BufferConsoleInner {
    fn write_char(&mut self, c: char) {
        if self.write_ptr < (BUF_SIZE - 1) {
            self.buf[self.write_ptr] = c;
            self.write_ptr += 1;
        }
    }
}

impl fmt::Write for BufferConsoleInner {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            self.write_char(c);
        }

        Ok(())
    }
}

impl BufferConsole {
    pub fn dump(&self) {
        self.inner.read(|inner| {
            console::console().write_array(&inner.buf[0..inner.write_ptr]);

            if inner.write_ptr == (BUF_SIZE - 1) {
                warn!("pre-UART buffer overflowed!");
            } else if inner.write_ptr > 0 {
                debug!("end of pre-UART buffer");
            }
        });
    }
}

impl interface::Write for BufferConsole {
    fn write_char(&self, c: char) {
        self.inner.write(|inner| inner.write_char(c));
    }

    fn write_array(&self, _a: &[char]) {
        unimplemented!("write_array is not implemented for BufferConsole");
    }

    fn write_fmt(&self, args: fmt::Arguments) -> fmt::Result {
        self.inner.write(|inner| inner.write_fmt(args))
    }

    fn flush(&self) {}
}

impl interface::Read for BufferConsole {
    fn clear_rx(&self) {}
}

impl interface::Statistics for BufferConsole {}
impl interface::All for BufferConsole {}
