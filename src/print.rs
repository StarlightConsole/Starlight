use crate::console;
use core::fmt;

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    console::console().write_fmt(args).unwrap();
}

/// Prints without a newline
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::print::_print(format_args!($($arg)*)));
}

/// Prints with a newline
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ({
        $crate::print::_print(format_args_nl!($($arg)*));
    })
}

/// Prints an info, with a newline
#[macro_export]
macro_rules! info {
    ($string:expr) => ({
        let timestamp = $crate::time::time_manager().uptime();

        $crate::print::_print(format_args_nl!(
            concat!("[I {:>3}.{:06}] ", $string),
            timestamp.as_secs(),
            timestamp.subsec_micros(),
        ));
    });
    ($format_string:expr, $($arg:tt)*) => ({
        let timestamp = $crate::time::time_manager().uptime();

        $crate::print::_print(format_args_nl!(
            concat!("[I {:>3}.{:06}] ", $format_string),
            timestamp.as_secs(),
            timestamp.subsec_micros(),
            $($arg)*
        ));
    })
}

/// Prints a warning, with a newline
#[macro_export]
macro_rules! warn {
    ($string:expr) => ({
        let timestamp = $crate::time::time_manager().uptime();

        $crate::print::_print(format_args_nl!(
            concat!("[W {:>3}.{:06}] ", $string),
            timestamp.as_secs(),
            timestamp.subsec_micros(),
        ));
    });
    ($format_string:expr, $($arg:tt)*) => ({
        let timestamp = $crate::time::time_manager().uptime();

        $crate::print::_print(format_args_nl!(
            concat!("[W {:>3}.{:06}] ", $format_string),
            timestamp.as_secs(),
            timestamp.subsec_micros(),
            $($arg)*
        ));
    })
}

/// Prints an error, with a newline
#[macro_export]
macro_rules! error {
    ($string:expr) => ({
        let timestamp = $crate::time::time_manager().uptime();

        $crate::print::_print(format_args_nl!(
            concat!("[E {:>3}.{:06}] ", $string),
            timestamp.as_secs(),
            timestamp.subsec_micros(),
        ));
    });
    ($format_string:expr, $($arg:tt)*) => ({
        let timestamp = $crate::time::time_manager().uptime();

        $crate::print::_print(format_args_nl!(
            concat!("[E {:>3}.{:06}] ", $format_string),
            timestamp.as_secs(),
            timestamp.subsec_micros(),
            $($arg)*
        ));
    })
}

/// Prints a debug message only shown when using the feature "debug_prints", with a newline
#[macro_export]
macro_rules! debug {
    ($string:expr) => ({
        if cfg!(feature = "debug_prints") {
            let timestamp = $crate::time::time_manager().uptime();

            $crate::print::_print(format_args_nl!(
                concat!("[D {:>3}.{:06}] ", $string),
                timestamp.as_secs(),
                timestamp.subsec_micros(),
            ));
        }
    });
    ($format_string:expr, $($arg:tt)*) => ({
        if cfg!(feature = "debug_prints") {
            let timestamp = $crate::time::time_manager().uptime();

            $crate::print::_print(format_args_nl!(
                concat!("[D {:>3}.{:06}] ", $format_string),
                timestamp.as_secs(),
                timestamp.subsec_micros(),
                $($arg)*
            ));
        }
    });
}
