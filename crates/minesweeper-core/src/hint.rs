//! Deductions a careful player could make from the revealed numbers alone.
//! Flags are the player's own guesses, so they are never used as evidence.

use std::collections::BTreeSet;

use crate::{Game, Position, Status, game::Cover};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hint {
    /// Nothing is revealed yet, and the first reveal is always safe.
    FirstReveal,
    /// This cell cannot hold a mine. It may carry a mistaken flag.
    Safe {
        at: Position,
        /// Every revealed number the deduction relied on, including earlier steps.
        clues: Vec<Position>,
        /// The deduction also relied on the total number of mines.
        mine_count: bool,
    },
    /// This cell must hold a mine and has no flag yet.
    Mine {
        at: Position,
        clues: Vec<Position>,
        mine_count: bool,
    },
    /// No unknown cell is certain either way; only a guess remains.
    Guess,
}

/// Search nodes per frontier region; a larger region stays undecided.
const NODE_BUDGET: usize = 200_000;

/// A revealed number reduced to the cells that are still undecided.
#[derive(Debug, Clone)]
struct Clue {
    origin: usize,
    cells: Vec<usize>,
    mines: usize,
    /// Already-settled neighbors that this clue was reduced by.
    settled: Vec<usize>,
}

/// What has been proven so far, and from which numbers.
struct Known {
    /// `Some(true)` is a certain mine, `Some(false)` a certain safe cell.
    value: Vec<Option<bool>>,
    why: Vec<BTreeSet<usize>>,
    by_count: Vec<bool>,
}

impl Known {
    fn new(cells: usize) -> Self {
        Self {
            value: vec![None; cells],
            why: vec![BTreeSet::new(); cells],
            by_count: vec![false; cells],
        }
    }

    /// Record a new fact, inheriting the evidence of every settled cell the
    /// clues were reduced by. Returns whether the fact was new.
    fn learn<'a>(
        &mut self,
        cell: usize,
        mine: bool,
        clues: impl IntoIterator<Item = &'a Clue>,
        extra: &[usize],
        by_count: bool,
    ) -> bool {
        if self.value[cell].is_some() {
            return false;
        }
        let mut why = BTreeSet::new();
        let mut count = by_count;
        let mut settled: Vec<usize> = extra.to_vec();
        for clue in clues {
            why.insert(clue.origin);
            settled.extend(&clue.settled);
        }
        for cell in settled {
            why.extend(self.why[cell].iter().copied());
            count |= self.by_count[cell];
        }
        self.value[cell] = Some(mine);
        self.why[cell] = why;
        self.by_count[cell] = count;
        true
    }
}

impl Game {
    /// The simplest certain deduction, preferring a safe cell to reveal.
    /// `None` once the game is over.
    pub fn hint(&self) -> Option<Hint> {
        match self.status() {
            Status::Won | Status::Lost => return None,
            Status::Ready => return Some(Hint::FirstReveal),
            Status::Playing => {}
        }
        let mut known = Known::new(self.config.cells());
        // Local rules first: they are what a player sees at a glance.
        while apply_local_rules(&self.clues(&known), &mut known) {}
        if let Some(hint) = self.pick(&known) {
            return Some(hint);
        }
        self.enumerate(&mut known);
        Some(self.pick(&known).unwrap_or(Hint::Guess))
    }

    fn is_unknown(&self, index: usize, known: &Known) -> bool {
        self.cover[index] != Cover::Revealed && known.value[index].is_none()
    }

    fn clues(&self, known: &Known) -> Vec<Clue> {
        (0..self.config.cells())
            .filter(|&index| self.cover[index] == Cover::Revealed)
            .filter_map(|index| {
                let hidden = self
                    .neighbors(index)
                    .filter(|&n| self.cover[n] != Cover::Revealed);
                let (settled, cells): (Vec<usize>, Vec<usize>) =
                    hidden.partition(|&n| known.value[n].is_some());
                let known_mines = settled
                    .iter()
                    .filter(|&&n| known.value[n] == Some(true))
                    .count();
                (!cells.is_empty()).then(|| Clue {
                    origin: index,
                    mines: usize::from(self.adjacent_mines(index)) - known_mines,
                    cells,
                    settled,
                })
            })
            .collect()
    }

    /// Safe cells to reveal, then mines to flag, then mistaken flags.
    fn pick(&self, known: &Known) -> Option<Hint> {
        let find = |mine: bool, cover: Cover| {
            (0..known.value.len()).find(|&i| known.value[i] == Some(mine) && self.cover[i] == cover)
        };
        let (index, mine) = find(false, Cover::Hidden)
            .map(|i| (i, false))
            .or_else(|| find(true, Cover::Hidden).map(|i| (i, true)))
            .or_else(|| find(false, Cover::Flagged).map(|i| (i, false)))?;
        let cols = self.config.cols;
        let at = Position::new(index / cols, index % cols);
        let clues = known.why[index]
            .iter()
            .map(|&n| Position::new(n / cols, n % cols))
            .collect();
        let mine_count = known.by_count[index];
        Some(if mine {
            Hint::Mine {
                at,
                clues,
                mine_count,
            }
        } else {
            Hint::Safe {
                at,
                clues,
                mine_count,
            }
        })
    }

    /// Try every arrangement of each frontier region, then use the total
    /// mine count for cells that touch no number.
    fn enumerate(&self, known: &mut Known) {
        let clues = self.clues(known);
        let cells = self.config.cells();
        let undecided = (0..cells).filter(|&i| self.is_unknown(i, known)).count();
        let known_mines: Vec<usize> = (0..cells)
            .filter(|&i| known.value[i] == Some(true))
            .collect();
        let remaining = self.config.mines - known_mines.len();
        let mut region_of = vec![usize::MAX; cells];
        let mut regions: Vec<Vec<usize>> = Vec::new();
        // Flood regions through shared clues, giving a search order in which
        // each clue's cells are assigned close together.
        for start in clues.iter().flat_map(|clue| clue.cells.iter().copied()) {
            if region_of[start] != usize::MAX {
                continue;
            }
            let id = regions.len();
            let mut members = vec![start];
            region_of[start] = id;
            let mut next = 0;
            while next < members.len() {
                let cell = members[next];
                next += 1;
                for clue in clues.iter().filter(|clue| clue.cells.contains(&cell)) {
                    for &other in &clue.cells {
                        if region_of[other] == usize::MAX {
                            region_of[other] = id;
                            members.push(other);
                        }
                    }
                }
            }
            regions.push(members);
        }
        let mut min_total = 0;
        let mut max_total = 0;
        let mut complete = true;
        for members in &regions {
            let region_clues: Vec<&Clue> = clues
                .iter()
                .filter(|clue| region_of[clue.cells[0]] == region_of[members[0]])
                .collect();
            let outside = undecided - members.len();
            let Some(result) = solve_region(members, &region_clues, remaining, outside) else {
                complete = false;
                continue;
            };
            min_total += result.min_mines;
            max_total += result.max_mines;
            // Rejecting arrangements by the mine count makes it evidence too.
            let extra: &[usize] = if result.used_count { &known_mines } else { &[] };
            for (k, &cell) in members.iter().enumerate() {
                let mine = match (result.can_be_mine[k], result.can_be_safe[k]) {
                    (true, false) => true,
                    (false, true) => false,
                    _ => continue,
                };
                known.learn(
                    cell,
                    mine,
                    region_clues.iter().copied(),
                    extra,
                    result.used_count,
                );
            }
        }
        let interior: Vec<usize> = (0..cells)
            .filter(|&i| self.is_unknown(i, known) && region_of[i] == usize::MAX)
            .collect();
        if complete && !interior.is_empty() {
            // Every mine not needed by the frontier must sit in the interior.
            let all = if min_total == remaining {
                Some(false)
            } else if max_total + interior.len() == remaining {
                Some(true)
            } else {
                None
            };
            if let Some(mine) = all {
                for cell in interior {
                    known.learn(cell, mine, &clues, &known_mines, true);
                }
            }
        }
    }
}

/// Single numbers and pairs of numbers whose cells nest. Returns whether
/// anything new was learned.
fn apply_local_rules(clues: &[Clue], known: &mut Known) -> bool {
    let mut learned = false;
    for clue in clues {
        let mine = if clue.mines == 0 {
            false
        } else if clue.mines == clue.cells.len() {
            true
        } else {
            continue;
        };
        for &cell in &clue.cells {
            learned |= known.learn(cell, mine, [clue], &[], false);
        }
    }
    for small in clues {
        for large in clues {
            if small.cells.len() >= large.cells.len()
                || !small.cells.iter().all(|cell| large.cells.contains(cell))
            {
                continue;
            }
            let rest: Vec<usize> = large
                .cells
                .iter()
                .copied()
                .filter(|c| !small.cells.contains(c))
                .collect();
            let mine = if large.mines == small.mines {
                false
            } else if large.mines.saturating_sub(small.mines) == rest.len() {
                true
            } else {
                continue;
            };
            for cell in rest {
                learned |= known.learn(cell, mine, [small, large], &[], false);
            }
        }
    }
    learned
}

struct RegionResult {
    can_be_mine: Vec<bool>,
    can_be_safe: Vec<bool>,
    min_mines: usize,
    max_mines: usize,
    /// Whether the board's mine count ruled out any arrangement.
    used_count: bool,
}

/// Exhaustive search over one region. Arrangements that leave too many or
/// too few mines for the rest of the board are rejected, which is sound
/// because those arrangements cannot happen.
fn solve_region(
    members: &[usize],
    clues: &[&Clue],
    remaining: usize,
    outside: usize,
) -> Option<RegionResult> {
    struct Search<'a> {
        clue_of: Vec<Vec<usize>>,
        need: Vec<usize>,
        placed: Vec<usize>,
        open: Vec<usize>,
        mine: Vec<bool>,
        result: &'a mut RegionResult,
        remaining: usize,
        outside: usize,
        nodes: usize,
        found: bool,
    }

    impl Search<'_> {
        fn run(&mut self, k: usize, mines: usize) -> bool {
            self.nodes += 1;
            if self.nodes > NODE_BUDGET {
                return false;
            }
            if mines > self.remaining {
                self.result.used_count = true;
                return true;
            }
            if k == self.mine.len() {
                if self.remaining - mines > self.outside {
                    self.result.used_count = true;
                } else {
                    self.found = true;
                    self.result.min_mines = self.result.min_mines.min(mines);
                    self.result.max_mines = self.result.max_mines.max(mines);
                    for (cell, &mine) in self.mine.iter().enumerate() {
                        if mine {
                            self.result.can_be_mine[cell] = true;
                        } else {
                            self.result.can_be_safe[cell] = true;
                        }
                    }
                }
                return true;
            }
            for mine in [false, true] {
                let clues = std::mem::take(&mut self.clue_of[k]);
                let mut fits = true;
                for &c in &clues {
                    self.open[c] -= 1;
                    self.placed[c] += usize::from(mine);
                    fits &= self.placed[c] <= self.need[c]
                        && self.placed[c] + self.open[c] >= self.need[c];
                }
                self.mine[k] = mine;
                let ok = !fits || self.run(k + 1, mines + usize::from(mine));
                for &c in &clues {
                    self.open[c] += 1;
                    self.placed[c] -= usize::from(mine);
                }
                self.clue_of[k] = clues;
                if !ok {
                    return false;
                }
            }
            true
        }
    }

    let mut result = RegionResult {
        can_be_mine: vec![false; members.len()],
        can_be_safe: vec![false; members.len()],
        min_mines: usize::MAX,
        max_mines: 0,
        used_count: false,
    };
    let mut search = Search {
        clue_of: members
            .iter()
            .map(|cell| {
                (0..clues.len())
                    .filter(|&c| clues[c].cells.contains(cell))
                    .collect()
            })
            .collect(),
        need: clues.iter().map(|clue| clue.mines).collect(),
        placed: vec![0; clues.len()],
        open: clues.iter().map(|clue| clue.cells.len()).collect(),
        mine: vec![false; members.len()],
        result: &mut result,
        remaining,
        outside,
        nodes: 0,
        found: false,
    };
    (search.run(0, 0) && search.found).then_some(result)
}
