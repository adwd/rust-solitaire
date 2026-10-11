use std::{error::Error, fmt};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{CELLS, Grid, history::History, solver};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Difficulty {
    Easy,
    #[default]
    Medium,
    Hard,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Medium, Self::Hard];

    /// Clue removal stops here; a puzzle never has fewer givens than this.
    pub const fn target_givens(self) -> usize {
        match self {
            Self::Easy => 40,
            Self::Medium => 32,
            Self::Hard => 26,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Easy => "Easy",
            Self::Medium => "Medium",
            Self::Hard => "Hard",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
    pub const fn index(self) -> usize {
        self.row * 9 + self.col
    }
    const fn from_index(index: usize) -> Self {
        Self::new(index / 9, index % 9)
    }
    fn in_range(self) -> bool {
        self.row < 9 && self.col < 9
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    /// Writing a digit clears the cell's notes and removes the digit from
    /// notes in the same row, column, and box.
    Set {
        at: Position,
        digit: u8,
    },
    Clear {
        at: Position,
    },
    ToggleNote {
        at: Position,
        digit: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    Solved,
    OutOfRange,
    InvalidDigit,
    GivenCell,
    CellFilled,
    Unchanged,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Solved => "the puzzle is already solved",
            Self::OutOfRange => "the cell is outside the grid",
            Self::InvalidDigit => "digits must be between 1 and 9",
            Self::GivenCell => "given cells cannot be changed",
            Self::CellFilled => "notes can only be added to empty cells",
            Self::Unchanged => "the cell already looks like that",
        })
    }
}

impl Error for MoveError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Playing,
    Solved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hint {
    pub at: Position,
    pub digit: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct State {
    cells: Grid,
    /// Bit `d` marks digit `d` as a pencil note.
    notes: [u16; CELLS],
    moves: u64,
}

#[derive(Debug)]
pub struct Game {
    state: State,
    history: History<State>,
    givens: Grid,
    solution: Grid,
    seed: u64,
    difficulty: Difficulty,
}

impl Game {
    /// Generator version 1: ChaCha8 fills a random solution, then clues are
    /// removed in u32 Fisher–Yates order while the solution stays unique.
    pub fn new(seed: u64, difficulty: Difficulty) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let solution = solver::random_solution(&mut rng);
        let mut order: Vec<usize> = (0..CELLS).collect();
        for upper in (1..order.len()).rev() {
            order.swap(upper, rng.random_range(0..=upper as u32) as usize);
        }
        let mut givens = solution;
        let mut count = CELLS;
        for index in order {
            if count <= difficulty.target_givens() {
                break;
            }
            let digit = std::mem::take(&mut givens[index]);
            if solver::solutions(&givens, 2).len() == 1 {
                count -= 1;
            } else {
                givens[index] = digit;
            }
        }
        Self::from_parts(givens, solution, seed, difficulty)
    }

    fn from_parts(givens: Grid, solution: Grid, seed: u64, difficulty: Difficulty) -> Self {
        Self {
            state: State {
                cells: givens,
                notes: [0; CELLS],
                moves: 0,
            },
            history: History::default(),
            givens,
            solution,
            seed,
            difficulty,
        }
    }

    /// Rows of digits with `.` or `0` for blanks; whitespace is ignored.
    #[cfg(test)]
    pub(crate) fn from_puzzle(text: &str) -> Option<Self> {
        let digits: Vec<u8> = text
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| {
                if c == '.' {
                    Some(0)
                } else {
                    c.to_digit(10).map(|d| d as u8)
                }
            })
            .collect::<Option<_>>()?;
        let givens: Grid = digits.try_into().ok()?;
        let [solution] = solver::solutions(&givens, 2).try_into().ok()?;
        Some(Self::from_parts(givens, solution, 0, Difficulty::default()))
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }
    pub fn moves(&self) -> u64 {
        self.state.moves
    }
    pub fn value(&self, at: Position) -> Option<u8> {
        Some(self.state.cells[at.index()]).filter(|&digit| digit != 0)
    }
    pub fn is_given(&self, at: Position) -> bool {
        self.givens[at.index()] != 0
    }
    pub fn has_note(&self, at: Position, digit: u8) -> bool {
        self.state.notes[at.index()] & (1 << digit) != 0
    }
    pub fn givens_count(&self) -> usize {
        self.givens.iter().filter(|&&digit| digit != 0).count()
    }
    pub fn filled_count(&self) -> usize {
        self.state.cells.iter().filter(|&&digit| digit != 0).count()
    }
    /// How many of a digit are on the grid, for "digit complete" displays.
    pub fn digit_count(&self, digit: u8) -> usize {
        self.state.cells.iter().filter(|&&d| d == digit).count()
    }

    /// A filled cell that repeats a digit in its row, column, or box.
    pub fn conflicts(&self, at: Position) -> bool {
        let index = at.index();
        let digit = self.state.cells[index];
        digit != 0 && peers(index).any(|peer| self.state.cells[peer] == digit)
    }

    /// Player-entered digits that differ from the unique solution.
    pub fn mistakes(&self) -> Vec<Position> {
        (0..CELLS)
            .filter(|&i| self.state.cells[i] != 0 && self.state.cells[i] != self.solution[i])
            .map(Position::from_index)
            .collect()
    }

    /// The solution digit for the empty cell with the fewest candidates.
    pub fn hint(&self) -> Option<Hint> {
        (0..CELLS)
            .filter(|&i| self.state.cells[i] == 0)
            .min_by_key(|&i| {
                (1..=9u8)
                    .filter(|&d| peers(i).all(|peer| self.state.cells[peer] != d))
                    .count()
            })
            .map(|i| Hint {
                at: Position::from_index(i),
                digit: self.solution[i],
            })
    }

    pub fn status(&self) -> Status {
        if self.state.cells == self.solution {
            Status::Solved
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

    pub fn validate(&self, action: Action) -> Result<(), MoveError> {
        if self.status() == Status::Solved {
            return Err(MoveError::Solved);
        }
        let (Action::Set { at, .. } | Action::Clear { at } | Action::ToggleNote { at, .. }) =
            action;
        if !at.in_range() {
            return Err(MoveError::OutOfRange);
        }
        if let Action::Set { digit, .. } | Action::ToggleNote { digit, .. } = action
            && !(1..=9).contains(&digit)
        {
            return Err(MoveError::InvalidDigit);
        }
        if self.is_given(at) {
            return Err(MoveError::GivenCell);
        }
        let index = at.index();
        match action {
            Action::Set { digit, .. } if self.state.cells[index] == digit => {
                Err(MoveError::Unchanged)
            }
            Action::Clear { .. }
                if self.state.cells[index] == 0 && self.state.notes[index] == 0 =>
            {
                Err(MoveError::Unchanged)
            }
            Action::ToggleNote { .. } if self.state.cells[index] != 0 => Err(MoveError::CellFilled),
            _ => Ok(()),
        }
    }

    /// Validation happens before any mutation, including history changes.
    pub fn apply(&mut self, action: Action) -> Result<Status, MoveError> {
        self.validate(action)?;
        self.history.record(self.state.clone());
        let state = &mut self.state;
        match action {
            Action::Set { at, digit } => {
                let index = at.index();
                state.cells[index] = digit;
                state.notes[index] = 0;
                for peer in peers(index) {
                    state.notes[peer] &= !(1 << digit);
                }
            }
            Action::Clear { at } => {
                state.cells[at.index()] = 0;
                state.notes[at.index()] = 0;
            }
            Action::ToggleNote { at, digit } => state.notes[at.index()] ^= 1 << digit,
        }
        state.moves += 1;
        Ok(self.status())
    }
}

/// The 20 other cells sharing a row, column, or box.
fn peers(index: usize) -> impl Iterator<Item = usize> {
    (0..CELLS).filter(move |&other| {
        other != index
            && (other / 9 == index / 9
                || other % 9 == index % 9
                || solver::box_of(other) == solver::box_of(index))
    })
}
