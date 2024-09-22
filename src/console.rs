mod null_console;

use crate::synchronization;

#[allow(unused)]
pub mod interface {
    use core::fmt;

    pub trait Write {
        fn write_char(&self, c: char);
        fn write_fmt(&self, args: fmt::Arguments) -> fmt::Result;
        #[allow(unused)]
        fn flush(&self);
    }

    pub trait Read {
        fn read_char(&self) -> char {
            ' '
        }

        fn clear_rx(&self);
    }

    pub trait Statistics {
        #[allow(unused)]
        fn chars_written(&self) -> usize {
            0
        }

        #[allow(unused)]
        fn chars_read(&self) -> usize {
            0
        }
    }
    
    pub trait All: Write + Read + Statistics {}
}

static CUR_CONSOLE: InitStateLock<&'static (dyn interface::All + Sync)> = InitStateLock::new(&null_console::NULL_CONSOLE);

use synchronization::{interface::ReadWriteEx, InitStateLock};

pub fn register_console(new_console: &'static (dyn interface::All + Sync)) {
    CUR_CONSOLE.write(|con| *con = new_console);
}

pub fn console() -> &'static dyn interface::All {
    CUR_CONSOLE.read(|con| *con)
}
