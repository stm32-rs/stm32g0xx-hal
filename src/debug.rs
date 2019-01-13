//! Provides debug output based on semihosting
//!
//! If you're running your program attached to a debugger, you might want to use
//! this module to enable debug output based on semihosting. This requires the
//! `debug` feature to be enabled.
//!
//! Without the `debug` feature, semihosting will not be initialized and
//! the macros in this module will not print anything. This makes it possible to
//! run your program without a debugger attached.
//!
//! ATTENTION: If you intend your program to run without a debugger attached,
//! always compile it without the `debug` feature. Programs that enable
//! semihosting cannot run without a debugger attached.

use core::cell::RefCell;

use cortex_m::interrupt::Mutex;
use cortex_m_semihosting::hio::HStdout;

/// Connects to the host's stdout
///
/// Users can typically ignore this static, and use [`init`], [`print!`], and
/// [`println!`] instead.
pub static STDOUT: Mutex<RefCell<Option<HStdout>>> = Mutex::new(RefCell::new(None));

/// Initializes the debug output, if semihosting is enabled
///
/// You should add a call to this function to the start any program that uses
/// [`print!`] or [`println!`]. If semihosting is enabled via the `debug`
/// feature, this function will initialize it. If the `debug` feature is
/// not enabled, this function does nothing.
pub fn init() {
    // Enable debug output only if the semihosting feature is enabled. We need
    // the option to disable semihosting, otherwise programs won't run without a
    // debugger attached.
    #[cfg(feature = "debug")]
    {
        use cortex_m::interrupt;
        use cortex_m_semihosting::hio;

        interrupt::free(|cs| {
            *STDOUT.borrow(cs).borrow_mut() =
                Some(hio::hstdout().expect("Failed to initialize semihosting"));
        });
    }
}

/// Sends a debug message to the host, if semihosting is enabled
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::cortex_m::interrupt::free(|cs| {
            if let Some(ref mut stdout) =
                *$crate::debug::STDOUT.borrow(cs).borrow_mut()
            {
                use core::fmt::Write;
                write!(stdout, $($arg)*).expect("Failed to write to stdout")
            }
        })
    }
}

/// Sends a debug message to the host, if semihosting is enabled
#[macro_export]
macro_rules! println {
    ($fmt:expr) => {
        print!(concat!($fmt, "\n"));
    };
    ($fmt:expr, $($arg:tt)*) => {
        print!(concat!($fmt, "\n"), $($arg)*);
    };
}
