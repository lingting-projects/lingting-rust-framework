#[cfg(debug_assertions)]
#[doc(hidden)]
#[macro_export]
macro_rules! __log_trace {
    ($($arg:tt)*) => {
        $crate::__private::log::trace!($($arg)*)
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! __log_trace {
    ($($arg:tt)*) => {{}};
}

#[cfg(debug_assertions)]
#[doc(hidden)]
#[macro_export]
macro_rules! __log_debug {
    ($($arg:tt)*) => {
        $crate::__private::log::debug!($($arg)*)
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! __log_debug {
    ($($arg:tt)*) => {{}};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log_info {
    ($($arg:tt)*) => {
        $crate::__private::log::info!($($arg)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log_warn {
    ($($arg:tt)*) => {
        $crate::__private::log::warn!($($arg)*)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __log_error {
    ($($arg:tt)*) => {
        $crate::__private::log::error!($($arg)*)
    };
}

pub use crate::__log_debug as log_debug;
pub use crate::__log_error as log_error;
pub use crate::__log_info as log_info;
pub use crate::__log_trace as log_trace;
pub use crate::__log_warn as log_warn;
