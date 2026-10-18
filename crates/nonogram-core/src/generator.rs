use super::{Game, Source, clues};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    #[default]
    Small,
    Large,
    ExtraLarge,
}

impl Size {
    pub const ALL: [Self; 3] = [Self::Small, Self::Large, Self::ExtraLarge];
    pub const fn side(self) -> usize {
        match self {
            Self::Small => 5,
            Self::Large => 10,
            Self::ExtraLarge => 15,
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Small => "5 x 5",
            Self::Large => "10 x 10",
            Self::ExtraLarge => "15 x 15",
        }
    }
}

fn patterns(side: usize, target: &[usize]) -> Vec<u16> {
    (0..1 << side)
        .filter(|mask| clues((0..side).map(|bit| mask & (1 << bit) != 0)) == target)
        .collect()
}

/// Constraint propagation followed by bounded search. `None` means that the
/// budget ran out, never that a puzzle is unique. Counts stop at two solutions.
fn count(mut rows: Vec<Vec<u16>>, mut cols: Vec<Vec<u16>>, budget: &mut usize) -> Option<usize> {
    if *budget == 0 {
        return None;
    }
    *budget -= 1;
    let side = rows.len();
    loop {
        let before: usize = rows.iter().chain(&cols).map(Vec::len).sum();
        for (r, row) in rows.iter_mut().enumerate() {
            row.retain(|mask| {
                cols.iter().enumerate().all(|(c, column)| {
                    column
                        .iter()
                        .any(|choice| (choice >> r) & 1 == (mask >> c) & 1)
                })
            });
        }
        for (c, column) in cols.iter_mut().enumerate() {
            column.retain(|mask| {
                rows.iter().enumerate().all(|(r, row)| {
                    row.iter()
                        .any(|choice| (choice >> c) & 1 == (mask >> r) & 1)
                })
            });
        }
        if rows.iter().chain(&cols).any(Vec::is_empty) {
            return Some(0);
        }
        let after: usize = rows.iter().chain(&cols).map(Vec::len).sum();
        if before == after {
            break;
        }
    }
    let branch = rows
        .iter()
        .chain(&cols)
        .enumerate()
        .filter(|(_, options)| options.len() > 1)
        .min_by_key(|(_, options)| options.len());
    let Some((index, choices)) = branch else {
        return Some(1);
    };
    let mut total = 0;
    for &choice in choices {
        let mut next_rows = rows.clone();
        let mut next_cols = cols.clone();
        if index < side {
            next_rows[index] = vec![choice];
        } else {
            next_cols[index - side] = vec![choice];
        }
        total += count(next_rows, next_cols, budget)?;
        if total >= 2 {
            return Some(2);
        }
    }
    Some(total)
}

fn unique(game: &Game) -> bool {
    let rows = game
        .row_clues()
        .iter()
        .map(|clue| patterns(game.size(), clue))
        .collect();
    let cols = game
        .column_clues()
        .iter()
        .map(|clue| patterns(game.size(), clue))
        .collect();
    count(rows, cols, &mut 2_000) == Some(1)
}

pub(super) fn generate(seed: u64, size: Size) -> Game {
    generate_with_attempts(seed, size, 64)
}

fn generate_with_attempts(seed: u64, size: Size, attempts: usize) -> Game {
    let side = size.side();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let source = Source::Random { seed, size };
    for _ in 0..attempts {
        let density = rng.random_range(42..=68u32);
        let picture = (0..side * side)
            .map(|_| rng.random_range(0..100u32) < density)
            .collect();
        let game = Game::from_solution(source, side, picture);
        if !game.is_won() && game.solution.iter().any(|&cell| !cell) && unique(&game) {
            return game;
        }
    }
    // Bounded fallback: nested, left-aligned runs. Full-height columns anchor
    // the picture, forcing each successively shorter column. Rotation and
    // reflection preserve uniqueness without giving up seed variation.
    let mut widths: Vec<_> = (0..side)
        .map(|_| rng.random_range(1..side as u32) as usize)
        .collect();
    widths.sort_unstable();
    let reflect = rng.random_range(0..2u32) == 0;
    let transpose = rng.random_range(0..2u32) == 0;
    let mut picture = vec![false; side * side];
    for (row, &width) in widths.iter().enumerate() {
        for col in 0..width {
            let col = if reflect { side - col - 1 } else { col };
            let (r, c) = if transpose { (col, row) } else { (row, col) };
            picture[r * side + c] = true;
        }
    }
    Game::from_solution(source, side, picture)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cell;

    #[test]
    fn bounded_fallback_is_also_uniquely_solvable() {
        for size in Size::ALL {
            for seed in 0..24 {
                let game = generate_with_attempts(seed, size, 0);
                let rows = game
                    .row_clues()
                    .iter()
                    .map(|clue| crate::tests::candidates(game.size(), clue))
                    .collect::<Vec<_>>();
                let cols = game
                    .column_clues()
                    .iter()
                    .map(|clue| crate::tests::candidates(game.size(), clue))
                    .collect::<Vec<_>>();
                assert_eq!(crate::tests::solution_count(&rows, &cols, 0), 1);
                assert!(!game.is_won());
            }
        }
    }

    #[test]
    fn solver_rejects_ambiguous_clues_and_budget_exhaustion() {
        assert_eq!(
            count(vec![vec![1, 2]; 2], vec![vec![1, 2]; 2], &mut 100),
            Some(2)
        );
        assert_eq!(
            count(vec![vec![1, 2]; 2], vec![vec![1, 2]; 2], &mut 0),
            None
        );
        assert_eq!(
            count(vec![vec![3], vec![0]], vec![vec![1]; 2], &mut 100),
            Some(1)
        );
    }

    #[test]
    fn generated_puzzles_are_unique_reproducible_and_restartable() {
        for size in Size::ALL {
            // A single 15 x 15 uniqueness check is much more expensive than
            // the smaller boards, so cover it with fewer seeds.
            let seeds = if size == Size::ExtraLarge { 6 } else { 48 };
            let mut pictures = std::collections::HashSet::new();
            for seed in 0..seeds {
                let mut game = Game::random(seed, size);
                assert_eq!(game.solution, Game::random(seed, size).solution);
                let rows = game
                    .row_clues()
                    .iter()
                    .map(|clue| crate::tests::candidates(game.size(), clue))
                    .collect::<Vec<_>>();
                let cols = game
                    .column_clues()
                    .iter()
                    .map(|clue| crate::tests::candidates(game.size(), clue))
                    .collect::<Vec<_>>();
                assert_eq!(
                    crate::tests::solution_count(&rows, &cols, 0),
                    1,
                    "seed {seed}, {size:?}"
                );
                pictures.insert(game.solution.clone());
                let solution = game.solution.clone();
                for (index, &filled) in solution.iter().enumerate() {
                    if filled {
                        assert!(game.apply(index / game.size(), index % game.size(), Cell::Filled));
                    }
                }
                assert!(game.is_won());
                game.restart();
                assert_eq!(game.solution, solution);
                assert!(!game.is_won());
                assert!(!game.can_undo() && !game.can_redo());
            }
            assert_eq!(pictures.len() as u64, seeds);
        }
    }
}
