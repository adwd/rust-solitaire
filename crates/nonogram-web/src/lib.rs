//! Nonogram screen, hosted by `games-web` and testable without a GPU.

mod app;
mod theme;

pub use app::NonogramApp;

/// Obtain a fresh seed from the host.
pub fn random_seed() -> u64 {
    rand::random()
}
