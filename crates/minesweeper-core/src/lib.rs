//! Deterministic Minesweeper rules without any GUI, GPU, clock, or I/O dependencies.
//!
//! ```
//! use minesweeper_core::{Action, CellView, Config, Game, Position, Status};
//! let mut game = Game::new(42, Config::BEGINNER);
//! let status = game.apply(Action::Reveal(Position::new(4, 4))).unwrap();
//! assert_ne!(status, Status::Lost, "the first reveal is always safe");
//! assert_eq!(game.cell(Position::new(4, 4)), CellView::Revealed(0));
//! ```

mod game;
mod hint;

pub use game::{Action, CellView, Config, ConfigError, Game, MoveError, Position, Status};
pub use hint::Hint;

#[cfg(test)]
mod tests;
