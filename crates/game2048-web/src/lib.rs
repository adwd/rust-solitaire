//! egui game screen for the browser, hosted by `games-web`. It also builds natively for tests.

mod app;
mod theme;

pub use app::Game2048App;

/// Obtain a fresh seed using the host's entropy source.
/// Deterministic generation remains entirely inside the core crate.
pub fn random_seed() -> u64 {
    rand::random()
}
