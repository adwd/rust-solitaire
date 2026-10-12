//! A browser host compiled once per game. The HTML shell lazily loads each
//! module and retains its runner, so switching preserves progress.

mod hub;
#[cfg(any(target_arch = "wasm32", test))]
mod screen;
#[cfg(target_arch = "wasm32")]
mod web;

pub use hub::GameKind;
