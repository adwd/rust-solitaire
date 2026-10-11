//! Browser host with a game switcher. Rules live in each core crate and
//! screens in each UI crate; this crate only chooses which screen is shown.

mod hub;
#[cfg(target_arch = "wasm32")]
mod web;

pub use hub::{GameKind, GamesApp};
