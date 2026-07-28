#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
mod native;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod wasm;

#[cfg(feature = "ntp")]
mod waiter;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use native::*;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use wasm::*;

#[cfg(feature = "ntp")]
pub(crate) use waiter::wake_waiters;
