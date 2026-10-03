//! Deterministic Klondike rules without any GUI, GPU, clock, or I/O dependencies.
//!
//! ```
//! use solitaire_core::{Action, Game, Rules};
//! let mut game = Game::new(42, Rules::default());
//! game.apply(Action::Draw).unwrap();
//! assert_eq!(game.view().stock.len(), 23);
//! assert!(game.undo());
//! assert_eq!(game.view().stock.len(), 24);
//! ```

mod action;
mod card;
mod game;
mod history;
mod rules;

pub use action::{Action, MoveError, Source, Target};
pub use card::{Card, Color, Rank, Suit};
pub use game::{DrawMode, Game, GameView, Rules, Status, TableauCard};

#[cfg(test)]
mod tests;
