use super::{Direction, Game, Position, Source, State, Tile};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    Small,
    #[default]
    Medium,
    Large,
}

impl Size {
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];
    pub const fn side(self) -> usize {
        match self {
            Self::Small => 7,
            Self::Medium => 9,
            Self::Large => 11,
        }
    }
    pub const fn crates(self) -> usize {
        match self {
            Self::Small => 2,
            Self::Medium => 3,
            Self::Large => 4,
        }
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::Small => "Small (7 x 7, 2 crates)",
            Self::Medium => "Medium (9 x 9, 3 crates)",
            Self::Large => "Large (11 x 11, 4 crates)",
        }
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Up => Direction::Down,
        Direction::Down => Direction::Up,
        Direction::Left => Direction::Right,
        Direction::Right => Direction::Left,
    }
}

/// Reachable squares and the walking path to each square, without moving crates.
fn walks(game: &Game) -> Vec<Option<Vec<Direction>>> {
    let mut paths = vec![None; game.tiles.len()];
    paths[game.player().row * game.width + game.player().col] = Some(Vec::new());
    let mut queue = VecDeque::from([game.player()]);
    while let Some(from) = queue.pop_front() {
        for direction in Direction::ALL {
            if let Some(to) = game.neighbor(from, direction) {
                let index = to.row * game.width + to.col;
                if paths[index].is_none() && !game.crates().contains(&to) {
                    let mut path = paths[from.row * game.width + from.col].clone().unwrap();
                    path.push(direction);
                    paths[index] = Some(path);
                    queue.push_back(to);
                }
            }
        }
    }
    paths
}

fn shuffle<T>(items: &mut [T], rng: &mut ChaCha8Rng) {
    for index in (1..items.len()).rev() {
        let other = rng.random_range(0..=index as u32) as usize;
        items.swap(index, other);
    }
}

fn connected(tiles: &[Tile], side: usize) -> bool {
    let Some(start) = tiles.iter().position(|&tile| tile == Tile::Floor) else {
        return false;
    };
    let mut seen = vec![false; tiles.len()];
    let mut queue = VecDeque::from([start]);
    seen[start] = true;
    while let Some(index) = queue.pop_front() {
        // The outside ring is all walls and is never added to the queue.
        for next in [index - side, index + side, index - 1, index + 1] {
            if tiles[next] == Tile::Floor && !seen[next] {
                seen[next] = true;
                queue.push_back(next);
            }
        }
    }
    tiles
        .iter()
        .enumerate()
        .all(|(i, &tile)| tile != Tile::Floor || seen[i])
}

/// Start with crates on goals, then walk and pull crates backwards. Reversing
/// this exact walk is a legal solution. Candidate selection favors crates far
/// from their original goals, rather than accepting an almost-solved scramble.
/// Fixed iteration bounds keep generation responsive, including in WASM.
pub(super) fn generate(seed: u64, size: Size) -> (Game, Vec<Direction>) {
    let side = size.side();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut best: Option<(usize, Game, Vec<Direction>)> = None;
    for attempt in 0..12 {
        let mut tiles = vec![Tile::Wall; side * side];
        for row in 1..side - 1 {
            for col in 1..side - 1 {
                // The first candidate is an open room, guaranteeing room to pull.
                if attempt == 0 || rng.random_range(0..8u32) != 0 {
                    tiles[row * side + col] = Tile::Floor;
                }
            }
        }
        if !connected(&tiles, side) {
            continue;
        }
        let interior_walls =
            tiles.iter().filter(|&&tile| tile == Tile::Wall).count() - (side * 4 - 4);
        let mut positions: Vec<_> = (2..side - 2)
            .flat_map(|row| (2..side - 2).map(move |col| Position { row, col }))
            .filter(|p| tiles[p.row * side + p.col] == Tile::Floor)
            .collect();
        if positions.len() < size.crates() + 1 {
            continue;
        }
        shuffle(&mut positions, &mut rng);
        let goals = positions[..size.crates()].to_vec();
        for p in &goals {
            tiles[p.row * side + p.col] = Tile::Goal;
        }
        let state = State {
            player: positions[size.crates()],
            crates: goals.clone(),
            moves: 0,
            pushes: 0,
        };
        let mut game = Game {
            source: Source::Random { seed, size },
            width: side,
            tiles,
            initial: state.clone(),
            state,
            past: Vec::new(),
            future: Vec::new(),
        };
        let mut reverse_walk = Vec::new();
        for _ in 0..(size.crates() * 70) {
            let paths = walks(&game);
            let mut options = Vec::new();
            for (index, &p) in game.crates().iter().enumerate() {
                for direction in Direction::ALL {
                    let Some(stand) = game.neighbor(p, direction) else {
                        continue;
                    };
                    let Some(destination) = game.neighbor(stand, direction) else {
                        continue;
                    };
                    if paths[stand.row * side + stand.col].is_some()
                        && !game.crates().contains(&destination)
                    {
                        options.push((index, direction, stand, destination));
                    }
                }
            }
            if options.is_empty() {
                break;
            }
            let (index, direction, stand, destination) =
                options[rng.random_range(0..options.len() as u32) as usize];
            reverse_walk.extend_from_slice(paths[stand.row * side + stand.col].as_ref().unwrap());
            reverse_walk.push(direction);
            game.state.crates[index] = stand;
            game.state.player = destination;
            let distance: usize = game
                .crates()
                .iter()
                .zip(&goals)
                .map(|(p, goal)| p.row.abs_diff(goal.row) + p.col.abs_diff(goal.col))
                .sum();
            let off_goals = game.crates().len() - game.crates_on_goals();
            let score = distance + off_goals * side + interior_walls * 3;
            if off_goals > 0
                && best
                    .as_ref()
                    .is_none_or(|(previous, _, _)| score > *previous)
            {
                let proof = reverse_walk.iter().rev().map(|&d| opposite(d)).collect();
                best = Some((score, game.clone(), proof));
            }
        }
    }
    let (_, mut game, proof) = best.expect("an open room always allows a pull");
    game.initial = game.state.clone();
    (game, proof)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_levels_are_solvable_reproducible_and_restartable() {
        for size in Size::ALL {
            for seed in 0..24 {
                let (mut game, proof) = generate(seed, size);
                let duplicate = Game::random(seed, size);
                assert_eq!(game.state, duplicate.state);
                assert_eq!(game.tiles, duplicate.tiles);
                assert_eq!(game.crates().len(), size.crates());
                assert!(!game.is_won());
                assert_eq!(game.moves(), 0);
                let initial = game.state.clone();
                for direction in proof {
                    if game.is_won() {
                        break;
                    }
                    assert!(game.apply(direction), "seed {seed}, {size:?}");
                }
                assert!(game.is_won(), "seed {seed}, {size:?}");
                while game.undo() {}
                assert_eq!(game.state, initial);
                while game.redo() {}
                assert!(game.is_won());
                game.restart();
                assert_eq!(game.state, initial);
                assert!(!game.can_undo() && !game.can_redo());
            }
        }
    }

    #[test]
    fn seeds_produce_distinct_layouts() {
        let layouts: std::collections::HashSet<_> = (0..24)
            .map(|seed| {
                let g = Game::random(seed, Size::Small);
                (
                    g.player(),
                    g.crates().to_vec(),
                    g.tiles.iter().map(|t| *t as u8).collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(layouts.len(), 24);
    }
}
