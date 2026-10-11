use std::{error::Error, fmt};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Config {
    pub rows: usize,
    pub cols: usize,
    pub mines: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    TooSmall,
    TooLarge,
    TooManyMines,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooSmall => "the board needs at least 2 cells and 1 mine",
            Self::TooLarge => "rows and columns are limited to 99",
            Self::TooManyMines => "at least one cell must be free of mines",
        })
    }
}

impl Error for ConfigError {}

impl Config {
    pub const BEGINNER: Self = Self {
        rows: 9,
        cols: 9,
        mines: 10,
    };
    pub const INTERMEDIATE: Self = Self {
        rows: 16,
        cols: 16,
        mines: 40,
    };
    pub const EXPERT: Self = Self {
        rows: 16,
        cols: 30,
        mines: 99,
    };

    pub fn custom(rows: usize, cols: usize, mines: usize) -> Result<Self, ConfigError> {
        if rows > 99 || cols > 99 {
            Err(ConfigError::TooLarge)
        } else if rows * cols < 2 || mines == 0 {
            Err(ConfigError::TooSmall)
        } else if mines >= rows * cols {
            Err(ConfigError::TooManyMines)
        } else {
            Ok(Self { rows, cols, mines })
        }
    }

    const fn cells(self) -> usize {
        self.rows * self.cols
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::BEGINNER
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Reveal(Position),
    ToggleFlag(Position),
    /// Reveal every unflagged neighbor of a number whose flags match it.
    Chord(Position),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    GameOver,
    OutOfRange,
    Revealed,
    Flagged,
    Hidden,
    FlagCountMismatch,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::GameOver => "the game is over",
            Self::OutOfRange => "the cell is outside the board",
            Self::Revealed => "the cell is already revealed",
            Self::Flagged => "remove the flag before revealing",
            Self::Hidden => "only revealed numbers can be chorded",
            Self::FlagCountMismatch => "the flags around this number do not match it",
        })
    }
}

impl Error for MoveError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// No cell has been revealed; mines are placed on the first reveal.
    Ready,
    Playing,
    Won,
    Lost,
}

/// What a player may see of a cell. Mines show only once the game ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellView {
    Hidden,
    Flagged,
    Revealed(u8),
    Mine,
    Exploded,
    WrongFlag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cover {
    Hidden,
    Flagged,
    Revealed,
}

#[derive(Debug, Clone)]
pub struct Game {
    config: Config,
    seed: u64,
    mines: Vec<bool>,
    cover: Vec<Cover>,
    placed: bool,
    exploded: Option<usize>,
    revealed: usize,
    flags: usize,
    moves: u64,
}

impl Game {
    pub fn new(seed: u64, config: Config) -> Self {
        Self {
            config,
            seed,
            mines: vec![false; config.cells()],
            cover: vec![Cover::Hidden; config.cells()],
            placed: false,
            exploded: None,
            revealed: 0,
            flags: 0,
            moves: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_mines(config: Config, mines: &[Position]) -> Self {
        let mut game = Self::new(0, config);
        for &at in mines {
            game.mines[at.row * config.cols + at.col] = true;
        }
        game.config.mines = mines.len();
        game.placed = true;
        game
    }

    #[cfg(test)]
    pub(crate) fn has_mine(&self, at: Position) -> bool {
        self.mines[at.row * self.config.cols + at.col]
    }

    pub fn config(&self) -> Config {
        self.config
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn moves(&self) -> u64 {
        self.moves
    }
    /// Mines minus flags, which goes negative when a player over-flags.
    pub fn mines_left(&self) -> isize {
        self.config.mines as isize - self.flags as isize
    }

    pub fn status(&self) -> Status {
        if self.exploded.is_some() {
            Status::Lost
        } else if self.revealed == self.config.cells() - self.config.mines {
            Status::Won
        } else if self.revealed == 0 {
            Status::Ready
        } else {
            Status::Playing
        }
    }

    pub fn cell(&self, at: Position) -> CellView {
        let index = self.index(at).expect("position inside the board");
        let over = matches!(self.status(), Status::Won | Status::Lost);
        match (self.cover[index], self.mines[index]) {
            // A chord can reveal more than one mine at once.
            (Cover::Revealed, true) => CellView::Exploded,
            (Cover::Revealed, false) => CellView::Revealed(self.adjacent_mines(index)),
            (Cover::Flagged, false) if over => CellView::WrongFlag,
            (Cover::Flagged, _) => CellView::Flagged,
            (Cover::Hidden, true) if self.status() == Status::Won => CellView::Flagged,
            (Cover::Hidden, true) if over => CellView::Mine,
            (Cover::Hidden, _) => CellView::Hidden,
        }
    }

    pub fn validate(&self, action: Action) -> Result<(), MoveError> {
        if matches!(self.status(), Status::Won | Status::Lost) {
            return Err(MoveError::GameOver);
        }
        let (Action::Reveal(at) | Action::ToggleFlag(at) | Action::Chord(at)) = action;
        let index = self.index(at).ok_or(MoveError::OutOfRange)?;
        match (action, self.cover[index]) {
            (Action::Reveal(_), Cover::Revealed) | (Action::ToggleFlag(_), Cover::Revealed) => {
                Err(MoveError::Revealed)
            }
            (Action::Reveal(_), Cover::Flagged) => Err(MoveError::Flagged),
            (Action::Chord(_), Cover::Hidden | Cover::Flagged) => Err(MoveError::Hidden),
            (Action::Chord(_), Cover::Revealed) => {
                let flags = self
                    .neighbors(index)
                    .filter(|&n| self.cover[n] == Cover::Flagged)
                    .count();
                if flags == usize::from(self.adjacent_mines(index)) {
                    Ok(())
                } else {
                    Err(MoveError::FlagCountMismatch)
                }
            }
            _ => Ok(()),
        }
    }

    /// Validation happens before any mutation.
    pub fn apply(&mut self, action: Action) -> Result<Status, MoveError> {
        self.validate(action)?;
        let (Action::Reveal(at) | Action::ToggleFlag(at) | Action::Chord(at)) = action;
        let index = self.index(at).expect("validated position");
        match action {
            Action::ToggleFlag(_) => {
                if self.cover[index] == Cover::Flagged {
                    self.cover[index] = Cover::Hidden;
                    self.flags -= 1;
                } else {
                    self.cover[index] = Cover::Flagged;
                    self.flags += 1;
                }
            }
            Action::Reveal(_) => {
                if !self.placed {
                    self.place_mines(index);
                }
                self.reveal(index);
            }
            Action::Chord(_) => {
                let targets: Vec<usize> = self
                    .neighbors(index)
                    .filter(|&n| self.cover[n] == Cover::Hidden)
                    .collect();
                for target in targets {
                    self.reveal(target);
                }
            }
        }
        self.moves += 1;
        Ok(self.status())
    }

    /// Placement version 1: the first revealed cell and, when room allows, its
    /// neighbors stay safe. Candidates are shuffled with ChaCha8 and u32
    /// Fisher–Yates, and the first `mines` cells receive mines.
    fn place_mines(&mut self, first: usize) {
        let mut safe: Vec<usize> = self.neighbors(first).collect();
        safe.push(first);
        if self.config.cells() - safe.len() < self.config.mines {
            safe = vec![first];
        }
        let mut candidates: Vec<usize> = (0..self.config.cells())
            .filter(|index| !safe.contains(index))
            .collect();
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);
        for upper in (1..candidates.len()).rev() {
            candidates.swap(upper, rng.random_range(0..=upper as u32) as usize);
        }
        for &index in &candidates[..self.config.mines] {
            self.mines[index] = true;
        }
        self.placed = true;
    }

    /// Flood-fills across zeros. Flags inside the flood stay in place.
    fn reveal(&mut self, start: usize) {
        if self.mines[start] {
            self.cover[start] = Cover::Revealed;
            self.exploded = Some(start);
            return;
        }
        let mut stack = vec![start];
        while let Some(index) = stack.pop() {
            if self.cover[index] != Cover::Hidden {
                continue;
            }
            self.cover[index] = Cover::Revealed;
            self.revealed += 1;
            if self.adjacent_mines(index) == 0 {
                stack.extend(self.neighbors(index));
            }
        }
    }

    fn index(&self, at: Position) -> Option<usize> {
        (at.row < self.config.rows && at.col < self.config.cols)
            .then_some(at.row * self.config.cols + at.col)
    }

    fn adjacent_mines(&self, index: usize) -> u8 {
        self.neighbors(index).filter(|&n| self.mines[n]).count() as u8
    }

    fn neighbors(&self, index: usize) -> impl Iterator<Item = usize> + use<> {
        let Config { rows, cols, .. } = self.config;
        let (row, col) = (index / cols, index % cols);
        (row.saturating_sub(1)..=(row + 1).min(rows - 1))
            .flat_map(move |r| {
                (col.saturating_sub(1)..=(col + 1).min(cols - 1)).map(move |c| (r, c))
            })
            .filter(move |&(r, c)| (r, c) != (row, col))
            .map(move |(r, c)| r * cols + c)
    }
}
