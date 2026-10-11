//! Deterministic 2048 rules without any GUI, GPU, clock, or I/O dependencies.
//!
//! ```
//! use game2048_core::{Direction, Game};
//! let mut game = Game::new(42);
//! let before = *game.board();
//! let direction = Direction::ALL
//!     .into_iter()
//!     .find(|&d| game.validate(d).is_ok())
//!     .unwrap();
//! game.apply(direction).unwrap();
//! assert!(game.undo());
//! assert_eq!(*game.board(), before);
//! ```

mod game;
mod history;

pub use game::{
    Board, Direction, Game, MoveError, Outcome, SIZE, Spawn, Status, WIN_EXPONENT, slide,
    slide_line,
};

#[cfg(test)]
mod tests;
