use std::{error::Error, fmt};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::history::History;

pub const SIZE: usize = 4;
/// Tiles are stored as exponents: `0` is empty, `1` is 2, `11` is 2048.
pub type Board = [[u8; SIZE]; SIZE];
pub const WIN_EXPONENT: u8 = 11;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const ALL: [Self; 4] = [Self::Up, Self::Down, Self::Left, Self::Right];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    GameOver,
    NoChange,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::GameOver => "no moves remain",
            Self::NoChange => "nothing can slide that way",
        })
    }
}

impl Error for MoveError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Playing,
    /// A 2048 tile exists; play may continue.
    Won,
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spawn {
    pub row: usize,
    pub col: usize,
    pub exponent: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub gained: u64,
    pub spawned: Spawn,
    pub status: Status,
}

/// The RNG lives in the state so undo and redo replay identical spawns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct State {
    board: Board,
    score: u64,
    moves: u64,
    rng: ChaCha8Rng,
}

#[derive(Debug)]
pub struct Game {
    state: State,
    history: History<State>,
    seed: u64,
}

impl Game {
    /// Spawn version 1: ChaCha8 picks an empty cell with a u32 range, then a
    /// 4 with probability 1/10 and a 2 otherwise. Two tiles start the game.
    pub fn new(seed: u64) -> Self {
        let mut state = State {
            board: [[0; SIZE]; SIZE],
            score: 0,
            moves: 0,
            rng: ChaCha8Rng::seed_from_u64(seed),
        };
        state.spawn();
        state.spawn();
        Self {
            state,
            history: History::default(),
            seed,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_board(board: Board, seed: u64) -> Self {
        Self {
            state: State {
                board,
                score: 0,
                moves: 0,
                rng: ChaCha8Rng::seed_from_u64(seed),
            },
            history: History::default(),
            seed,
        }
    }

    pub fn board(&self) -> &Board {
        &self.state.board
    }
    pub fn score(&self) -> u64 {
        self.state.score
    }
    pub fn moves(&self) -> u64 {
        self.state.moves
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn max_exponent(&self) -> u8 {
        self.state
            .board
            .iter()
            .flatten()
            .copied()
            .max()
            .unwrap_or(0)
    }

    pub fn status(&self) -> Status {
        if Direction::ALL
            .into_iter()
            .all(|direction| slide(&self.state.board, direction).0 == self.state.board)
        {
            Status::Over
        } else if self.max_exponent() >= WIN_EXPONENT {
            Status::Won
        } else {
            Status::Playing
        }
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
    pub fn undo(&mut self) -> bool {
        self.history.undo(&mut self.state)
    }
    pub fn redo(&mut self) -> bool {
        self.history.redo(&mut self.state)
    }

    pub fn validate(&self, direction: Direction) -> Result<(), MoveError> {
        if slide(&self.state.board, direction).0 != self.state.board {
            Ok(())
        } else if self.status() == Status::Over {
            Err(MoveError::GameOver)
        } else {
            Err(MoveError::NoChange)
        }
    }

    /// Validation happens before any mutation, including history changes.
    pub fn apply(&mut self, direction: Direction) -> Result<Outcome, MoveError> {
        self.validate(direction)?;
        self.history.record(self.state.clone());
        let (board, gained) = slide(&self.state.board, direction);
        self.state.board = board;
        self.state.score += gained;
        self.state.moves += 1;
        let spawned = self.state.spawn().expect("a changed board has a free cell");
        Ok(Outcome {
            gained,
            spawned,
            status: self.status(),
        })
    }
}

impl State {
    fn spawn(&mut self) -> Option<Spawn> {
        let empty: Vec<(usize, usize)> = (0..SIZE)
            .flat_map(|row| (0..SIZE).map(move |col| (row, col)))
            .filter(|&(row, col)| self.board[row][col] == 0)
            .collect();
        if empty.is_empty() {
            return None;
        }
        let (row, col) = empty[self.rng.random_range(0..empty.len() as u32) as usize];
        let exponent = if self.rng.random_range(0..10u32) == 0 {
            2
        } else {
            1
        };
        self.board[row][col] = exponent;
        Some(Spawn { row, col, exponent })
    }
}

/// Slide toward index 0, merging each pair at most once. Returns the score gained.
pub fn slide_line(line: [u8; SIZE]) -> ([u8; SIZE], u64) {
    let mut out = [0; SIZE];
    let mut gained = 0;
    let mut next = 0;
    let mut pending: Option<u8> = None;
    for tile in line.into_iter().filter(|&tile| tile != 0) {
        match pending {
            Some(previous) if previous == tile => {
                out[next] = tile + 1;
                gained += 1u64 << (tile + 1);
                next += 1;
                pending = None;
            }
            Some(previous) => {
                out[next] = previous;
                next += 1;
                pending = Some(tile);
            }
            None => pending = Some(tile),
        }
    }
    if let Some(previous) = pending {
        out[next] = previous;
    }
    (out, gained)
}

pub fn slide(board: &Board, direction: Direction) -> (Board, u64) {
    // Map line `i`, position `k` (k = 0 is the edge tiles slide toward) to a cell.
    let cell = |i: usize, k: usize| match direction {
        Direction::Left => (i, k),
        Direction::Right => (i, SIZE - 1 - k),
        Direction::Up => (k, i),
        Direction::Down => (SIZE - 1 - k, i),
    };
    let mut out = [[0; SIZE]; SIZE];
    let mut gained = 0;
    for i in 0..SIZE {
        let (line, score) = slide_line(std::array::from_fn(|k| {
            let (row, col) = cell(i, k);
            board[row][col]
        }));
        gained += score;
        for (k, tile) in line.into_iter().enumerate() {
            let (row, col) = cell(i, k);
            out[row][col] = tile;
        }
    }
    (out, gained)
}
