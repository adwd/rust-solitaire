//! Shared game screen and interactions for native and browser hosts.

mod app;
mod board;
mod input;
mod theme;

pub use app::SolitaireApp;

/// Obtain a fresh deal seed using the host's entropy source.
/// Deterministic shuffling remains entirely inside `solitaire-core`.
pub fn random_seed() -> u64 {
    rand::random()
}
