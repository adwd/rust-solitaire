use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::{CELLS, Grid};

const ALL: u16 = 0b11_1111_1110;

pub(crate) const fn box_of(index: usize) -> usize {
    (index / 27) * 3 + (index % 9) / 3
}

/// Digits are stored as bit `d` in each row, column, and box mask.
struct Search {
    cells: Grid,
    rows: [u16; 9],
    cols: [u16; 9],
    boxes: [u16; 9],
}

impl Search {
    fn new(grid: &Grid) -> Option<Self> {
        let mut search = Self {
            cells: [0; CELLS],
            rows: [0; 9],
            cols: [0; 9],
            boxes: [0; 9],
        };
        for (index, &digit) in grid.iter().enumerate() {
            if digit != 0 {
                if !(1..=9).contains(&digit) || search.candidates(index) & (1 << digit) == 0 {
                    return None;
                }
                search.place(index, digit);
            }
        }
        Some(search)
    }

    fn candidates(&self, index: usize) -> u16 {
        ALL & !(self.rows[index / 9] | self.cols[index % 9] | self.boxes[box_of(index)])
    }

    fn place(&mut self, index: usize, digit: u8) {
        let bit = 1 << digit;
        self.cells[index] = digit;
        self.rows[index / 9] |= bit;
        self.cols[index % 9] |= bit;
        self.boxes[box_of(index)] |= bit;
    }

    fn remove(&mut self, index: usize) {
        let bit = !(1u16 << self.cells[index]);
        self.cells[index] = 0;
        self.rows[index / 9] &= bit;
        self.cols[index % 9] &= bit;
        self.boxes[box_of(index)] &= bit;
    }

    /// Depth-first search on the most constrained cell. Shuffles candidates
    /// when an RNG is supplied, which turns an empty grid into a random fill.
    fn run(&mut self, limit: usize, mut rng: Option<&mut ChaCha8Rng>, found: &mut Vec<Grid>) {
        let mut best: Option<(usize, u16)> = None;
        for index in (0..CELLS).filter(|&index| self.cells[index] == 0) {
            let candidates = self.candidates(index);
            if best.is_none_or(|(_, mask)| candidates.count_ones() < mask.count_ones()) {
                best = Some((index, candidates));
                if candidates.count_ones() <= 1 {
                    break;
                }
            }
        }
        let Some((index, mask)) = best else {
            found.push(self.cells);
            return;
        };
        let mut digits: Vec<u8> = (1..=9).filter(|digit| mask & (1 << digit) != 0).collect();
        if let Some(rng) = rng.as_deref_mut() {
            for upper in (1..digits.len()).rev() {
                digits.swap(upper, rng.random_range(0..=upper as u32) as usize);
            }
        }
        for digit in digits {
            self.place(index, digit);
            self.run(limit, rng.as_deref_mut(), found);
            self.remove(index);
            if found.len() >= limit {
                return;
            }
        }
    }
}

/// Up to `limit` solutions of a grid; an inconsistent grid has none.
pub fn solutions(grid: &Grid, limit: usize) -> Vec<Grid> {
    let mut found = Vec::new();
    if let Some(mut search) = Search::new(grid) {
        search.run(limit, None, &mut found);
    }
    found
}

pub(crate) fn random_solution(rng: &mut ChaCha8Rng) -> Grid {
    let mut found = Vec::new();
    Search::new(&[0; CELLS])
        .expect("an empty grid is consistent")
        .run(1, Some(rng), &mut found);
    found.pop().expect("an empty grid has a solution")
}
