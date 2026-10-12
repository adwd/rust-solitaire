use super::*;
use std::collections::{HashSet, VecDeque};

/// Independent breadth-first search over positions, without using Game::apply.
fn solve(game: &Game) -> Vec<Direction> {
    let mut queue = VecDeque::from([(game.player(), game.crates().to_vec(), Vec::new())]);
    let mut seen = HashSet::new();
    while let Some((player, mut boxes, path)) = queue.pop_front() {
        boxes.sort_by_key(|p| (p.row, p.col));
        if !seen.insert((player, boxes.clone())) {
            continue;
        }
        assert!(
            seen.len() < 500_000,
            "level {:?} search limit",
            game.level()
        );
        if boxes.iter().all(|&p| game.tile(p) == Tile::Goal) {
            return path;
        }
        for direction in Direction::ALL {
            let step = |p: Position| -> Option<Position> {
                let (dr, dc) = match direction {
                    Direction::Up => (-1, 0),
                    Direction::Down => (1, 0),
                    Direction::Left => (0, -1),
                    Direction::Right => (0, 1),
                };
                let next = Position {
                    row: p.row.checked_add_signed(dr)?,
                    col: p.col.checked_add_signed(dc)?,
                };
                (game.tile(next) != Tile::Wall).then_some(next)
            };
            let Some(next) = step(player) else {
                continue;
            };
            let mut moved = boxes.clone();
            if let Some(index) = boxes.iter().position(|&p| p == next) {
                let Some(beyond) = step(next) else {
                    continue;
                };
                if boxes.contains(&beyond) {
                    continue;
                }
                moved[index] = beyond;
            }
            let mut next_path = path.clone();
            next_path.push(direction);
            queue.push_back((next, moved, next_path));
        }
    }
    panic!("level {:?} is unsolvable", game.level());
}

#[test]
fn every_level_is_valid_solvable_and_history_round_trips() {
    for (level, (_, map)) in levels::LEVELS.iter().enumerate() {
        let mut game = Game::new(level).unwrap();
        assert!(map.lines().all(|row| row.len() == game.width()));
        assert_eq!(map.chars().filter(|ch| matches!(ch, '@' | '+')).count(), 1);
        assert_eq!(
            game.crates().len(),
            game.tiles.iter().filter(|&&t| t == Tile::Goal).count()
        );
        assert!(!game.crates().is_empty());
        assert!(!game.is_won());
        let start = game.state.clone();
        let solution = solve(&game);
        for direction in &solution {
            assert!(game.apply(*direction));
        }
        assert!(game.is_won());
        assert_eq!(game.moves(), solution.len() as u64);
        assert!(game.pushes() > 0);
        let won = game.state.clone();
        assert!(!game.apply(Direction::Up));
        for _ in &solution {
            assert!(game.undo());
        }
        assert_eq!(game.state, start);
        assert!(!game.undo());
        for _ in &solution {
            assert!(game.redo());
        }
        assert_eq!(game.state, won);
        assert!(!game.redo());
        game.restart();
        assert_eq!(game.state, start);
        assert!(!game.can_undo() && !game.can_redo());
    }
    assert!(Game::new(LEVEL_COUNT).is_none());
}

#[test]
fn cannot_push_two_crates_or_push_through_a_wall() {
    let mut game = Game::default();
    game.state.crates = vec![Position { row: 2, col: 3 }, Position { row: 2, col: 4 }];
    let before = game.state.clone();
    assert!(!game.apply(Direction::Right));
    assert_eq!(game.state, before);
    assert!(!game.can_undo());
    game.state.crates = vec![Position { row: 1, col: 3 }];
    game.state.player = Position { row: 1, col: 2 };
    assert!(game.apply(Direction::Right));
    assert!(game.apply(Direction::Right));
    let before = game.state.clone();
    assert!(!game.apply(Direction::Right));
    assert_eq!(game.state, before);
}

#[test]
fn invalid_moves_preserve_redo_and_a_new_move_clears_it() {
    let mut game = Game::default();
    assert!(game.apply(Direction::Up));
    assert!(game.apply(Direction::Right));
    assert!(game.undo());
    let before = game.state.clone();
    assert!(!game.apply(Direction::Up));
    assert_eq!(game.state, before);
    assert!(game.can_redo());
    assert!(game.apply(Direction::Left));
    assert!(!game.can_redo());
    assert_eq!(
        game.tile(Position {
            row: usize::MAX,
            col: usize::MAX
        }),
        Tile::Wall
    );
}

#[test]
fn walking_does_not_pull_a_crate_and_goals_remain_underneath() {
    let mut game = Game::default();
    assert!(game.apply(Direction::Right));
    let boxes = game.crates().to_vec();
    assert!(game.apply(Direction::Left));
    assert_eq!(game.crates(), boxes);
    assert_eq!(game.pushes(), 1);
    assert!(game.apply(Direction::Right));
    assert!(game.apply(Direction::Right));
    assert!(game.is_won());
    assert!(game.undo());
    assert_eq!(game.tile(Position { row: 2, col: 5 }), Tile::Goal);
    assert!(!game.is_won());
}
