//! Nonogram clues, authored puzzles, marks, completion, and undo/redo.
//! Each built-in puzzle has exactly one solution, checked independently in tests.

mod generator;
mod puzzles;

pub use generator::Size;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Puzzle(usize),
    Random { seed: u64, size: Size },
}

pub const PUZZLE_COUNT: usize = puzzles::PUZZLES.len();

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Cell {
    #[default]
    Unknown,
    Filled,
    Crossed,
}

/// Consecutive filled runs, with `[0]` representing an entirely empty line.
pub fn clues(line: impl IntoIterator<Item = bool>) -> Vec<usize> {
    let mut result = Vec::new();
    let mut run = 0;
    for filled in line {
        if filled {
            run += 1;
        } else if run > 0 {
            result.push(run);
            run = 0;
        }
    }
    if run > 0 {
        result.push(run);
    }
    if result.is_empty() {
        result.push(0);
    }
    result
}

/// Minimum line length that can hold these runs: the filled squares plus
/// the mandatory gap between each pair of runs. `[0]` spans no squares.
pub fn clue_span(clue: &[usize]) -> usize {
    clue.iter().sum::<usize>() + clue.len().saturating_sub(1)
}

pub fn puzzle_size(puzzle: usize) -> Option<usize> {
    puzzles::PUZZLES.get(puzzle).map(|(_, rows)| rows.len())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    cells: Vec<Cell>,
    moves: u64,
}

#[derive(Debug, Clone)]
pub struct Game {
    source: Source,
    size: usize,
    solution: Vec<bool>,
    row_clues: Vec<Vec<usize>>,
    column_clues: Vec<Vec<usize>>,
    state: State,
    past: Vec<State>,
    future: Vec<State>,
}

impl Game {
    /// Puzzle indices are zero-based. Unknown puzzles are rejected.
    pub fn new(puzzle: usize) -> Option<Self> {
        let (_, rows) = puzzles::PUZZLES.get(puzzle)?;
        let solution: Vec<_> = rows
            .iter()
            .flat_map(|row| row.chars().map(|ch| ch == '#'))
            .collect();
        Some(Self::from_solution(
            Source::Puzzle(puzzle),
            rows.len(),
            solution,
        ))
    }

    fn from_solution(source: Source, size: usize, solution: Vec<bool>) -> Self {
        let row_clues = solution
            .chunks(size)
            .map(|row| clues(row.iter().copied()))
            .collect();
        let column_clues = (0..size)
            .map(|col| clues((0..size).map(|row| solution[row * size + col])))
            .collect();
        Self {
            source,
            size,
            solution,
            row_clues,
            column_clues,
            state: State {
                cells: vec![Cell::Unknown; size * size],
                moves: 0,
            },
            past: Vec::new(),
            future: Vec::new(),
        }
    }

    pub fn random(seed: u64, size: Size) -> Self {
        generator::generate(seed, size)
    }

    pub fn source(&self) -> Source {
        self.source
    }

    pub fn puzzle(&self) -> Option<usize> {
        match self.source {
            Source::Puzzle(puzzle) => Some(puzzle),
            Source::Random { .. } => None,
        }
    }
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn name(&self) -> &'static str {
        match self.source {
            Source::Puzzle(puzzle) => puzzles::PUZZLES[puzzle].0,
            Source::Random { .. } => "Random pattern",
        }
    }
    pub fn row_clues(&self) -> &[Vec<usize>] {
        &self.row_clues
    }
    pub fn column_clues(&self) -> &[Vec<usize>] {
        &self.column_clues
    }
    pub fn moves(&self) -> u64 {
        self.state.moves
    }

    pub fn cell(&self, row: usize, col: usize) -> Option<Cell> {
        if row < self.size && col < self.size {
            Some(self.state.cells[row * self.size + col])
        } else {
            None
        }
    }

    /// Empty squares need not be crossed for completion.
    pub fn is_won(&self) -> bool {
        self.state
            .cells
            .iter()
            .zip(&self.solution)
            .all(|(&cell, &filled)| (cell == Cell::Filled) == filled)
    }

    /// Compare the player's visible filled runs to a line's clues. This does
    /// not reveal whether its individual squares match the hidden picture.
    pub fn row_complete(&self, row: usize) -> bool {
        row < self.size
            && clues((0..self.size).map(|col| self.cell(row, col) == Some(Cell::Filled)))
                == self.row_clues[row]
    }

    pub fn column_complete(&self, col: usize) -> bool {
        col < self.size
            && clues((0..self.size).map(|row| self.cell(row, col) == Some(Cell::Filled)))
                == self.column_clues[col]
    }

    /// Invalid, unchanged, or post-completion edits do not alter history.
    pub fn apply(&mut self, row: usize, col: usize, cell: Cell) -> bool {
        if self.is_won() || self.cell(row, col).is_none_or(|before| before == cell) {
            return false;
        }
        self.past.push(self.state.clone());
        self.future.clear();
        self.state.cells[row * self.size + col] = cell;
        self.state.moves += 1;
        true
    }

    /// Applies several marks as one history entry, so a drag can be undone in
    /// a single step. Out-of-range cells and cells that already hold the
    /// requested mark are ignored.
    pub fn apply_batch(&mut self, edits: impl IntoIterator<Item = (usize, usize, Cell)>) -> bool {
        if self.is_won() {
            return false;
        }
        let mut next = self.state.clone();
        let mut changed = 0;
        for (row, col, cell) in edits {
            if row >= self.size || col >= self.size {
                continue;
            }
            let index = row * self.size + col;
            if next.cells[index] != cell {
                next.cells[index] = cell;
                changed += 1;
            }
        }
        if changed == 0 {
            return false;
        }
        next.moves += changed;
        self.past.push(self.state.clone());
        self.future.clear();
        self.state = next;
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.past.pop() {
            self.future
                .push(std::mem::replace(&mut self.state, previous));
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.future.pop() {
            self.past.push(std::mem::replace(&mut self.state, next));
            true
        } else {
            false
        }
    }

    pub fn restart(&mut self) {
        self.state.cells.fill(Cell::Unknown);
        self.state.moves = 0;
        self.past.clear();
        self.future.clear();
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new(0).expect("first puzzle")
    }
}

#[cfg(test)]
mod tests;
