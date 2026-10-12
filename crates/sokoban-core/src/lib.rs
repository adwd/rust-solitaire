//! Sokoban rules with authored, solvable levels and reversible moves.
//! No GUI, clock, or I/O dependencies. Generated levels are seed-reproducible.

mod generator;
mod levels;

pub use generator::Size;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Level(usize),
    Random { seed: u64, size: Size },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
pub enum Tile {
    Wall,
    Floor,
    Goal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    player: Position,
    crates: Vec<Position>,
    moves: u64,
    pushes: u64,
}

#[derive(Debug, Clone)]
pub struct Game {
    source: Source,
    width: usize,
    tiles: Vec<Tile>,
    state: State,
    initial: State,
    past: Vec<State>,
    future: Vec<State>,
}

pub const LEVEL_COUNT: usize = levels::LEVELS.len();

pub fn level_name(level: usize) -> Option<&'static str> {
    levels::LEVELS.get(level).map(|entry| entry.0)
}

impl Game {
    /// Level indices are zero-based. Unknown levels are rejected.
    pub fn new(level: usize) -> Option<Self> {
        let (_, map) = levels::LEVELS.get(level)?;
        let width = map.lines().next()?.len();
        let mut tiles = Vec::new();
        let mut crates = Vec::new();
        let mut player = Position { row: 0, col: 0 };
        for (row, line) in map.lines().enumerate() {
            for (col, ch) in line.chars().enumerate() {
                let position = Position { row, col };
                tiles.push(match ch {
                    '#' => Tile::Wall,
                    '.' | '*' | '+' => Tile::Goal,
                    _ => Tile::Floor,
                });
                if ch == '$' || ch == '*' {
                    crates.push(position);
                }
                if ch == '@' || ch == '+' {
                    player = position;
                }
            }
        }
        let state = State {
            player,
            crates,
            moves: 0,
            pushes: 0,
        };
        Some(Self {
            source: Source::Level(level),
            width,
            tiles,
            initial: state.clone(),
            state,
            past: Vec::new(),
            future: Vec::new(),
        })
    }

    pub fn random(seed: u64, size: Size) -> Self {
        generator::generate(seed, size).0
    }

    pub fn source(&self) -> Source {
        self.source
    }

    pub fn level(&self) -> Option<usize> {
        match self.source {
            Source::Level(level) => Some(level),
            Source::Random { .. } => None,
        }
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.tiles.len() / self.width
    }
    pub fn player(&self) -> Position {
        self.state.player
    }
    pub fn crates(&self) -> &[Position] {
        &self.state.crates
    }
    pub fn moves(&self) -> u64 {
        self.state.moves
    }
    pub fn pushes(&self) -> u64 {
        self.state.pushes
    }

    /// Out-of-bounds positions behave as walls.
    pub fn tile(&self, position: Position) -> Tile {
        if position.row >= self.height() || position.col >= self.width {
            Tile::Wall
        } else {
            self.tiles[position.row * self.width + position.col]
        }
    }

    pub fn crates_on_goals(&self) -> usize {
        self.crates()
            .iter()
            .filter(|&&p| self.tile(p) == Tile::Goal)
            .count()
    }

    pub fn is_won(&self) -> bool {
        self.crates_on_goals() == self.crates().len()
    }

    fn neighbor(&self, p: Position, direction: Direction) -> Option<Position> {
        let next = match direction {
            Direction::Up => Position {
                row: p.row.checked_sub(1)?,
                ..p
            },
            Direction::Down => Position {
                row: p.row.checked_add(1)?,
                ..p
            },
            Direction::Left => Position {
                col: p.col.checked_sub(1)?,
                ..p
            },
            Direction::Right => Position {
                col: p.col.checked_add(1)?,
                ..p
            },
        };
        (self.tile(next) != Tile::Wall).then_some(next)
    }

    /// Move one square, optionally pushing one crate. Blocked moves and moves
    /// after completion leave counters and both history stacks untouched.
    pub fn apply(&mut self, direction: Direction) -> bool {
        if self.is_won() {
            return false;
        }
        let Some(next) = self.neighbor(self.player(), direction) else {
            return false;
        };
        let pushed = self.crates().iter().position(|&p| p == next);
        let destination = if pushed.is_some() {
            let Some(beyond) = self.neighbor(next, direction) else {
                return false;
            };
            if self.crates().contains(&beyond) {
                return false;
            }
            Some(beyond)
        } else {
            None
        };
        self.past.push(self.state.clone());
        self.future.clear();
        self.state.player = next;
        self.state.moves += 1;
        if let (Some(index), Some(destination)) = (pushed, destination) {
            self.state.crates[index] = destination;
            self.state.pushes += 1;
        }
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
        self.state = self.initial.clone();
        self.past.clear();
        self.future.clear();
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new(0).expect("first level")
    }
}

#[cfg(test)]
mod tests;
