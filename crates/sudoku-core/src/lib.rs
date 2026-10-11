//! Deterministic Sudoku generation and rules without any GUI, GPU, clock, or I/O dependencies.
//!
//! ```
//! use sudoku_core::{Action, Difficulty, Game, Position};
//! let mut game = Game::new(42, Difficulty::Easy);
//! let hint = game.hint().unwrap();
//! game.apply(Action::Set { at: hint.at, digit: hint.digit }).unwrap();
//! assert_eq!(game.value(hint.at), Some(hint.digit));
//! assert!(game.undo());
//! assert_eq!(game.value(hint.at), None);
//! # let _ = Position::new(0, 0);
//! ```

mod game;
mod history;
mod solver;

pub use game::{Action, Difficulty, Game, Hint, MoveError, Position, Status};
pub use solver::solutions;

pub const CELLS: usize = 81;
/// Row-major digits, with `0` for an empty cell.
pub type Grid = [u8; CELLS];

#[cfg(test)]
mod tests;
