use crate::*;
use proptest::prelude::*;

#[test]
fn lines_merge_each_pair_once_toward_the_edge() {
    let cases = [
        ([1, 1, 1, 1], [2, 2, 0, 0], 8),
        ([1, 1, 2, 0], [2, 2, 0, 0], 4),
        ([0, 2, 0, 2], [3, 0, 0, 0], 8),
        ([2, 1, 1, 0], [2, 2, 0, 0], 4),
        ([1, 2, 3, 4], [1, 2, 3, 4], 0),
        ([0, 0, 0, 1], [1, 0, 0, 0], 0),
        ([3, 3, 3, 0], [4, 3, 0, 0], 16),
    ];
    for (line, expected, gained) in cases {
        assert_eq!(slide_line(line), (expected, gained), "{line:?}");
    }
}

#[test]
fn directions_map_to_the_matching_edge() {
    let board = [[1, 0, 0, 1], [0, 0, 0, 0], [0, 2, 0, 0], [0, 2, 0, 0]];
    assert_eq!(slide(&board, Direction::Left).0[0], [2, 0, 0, 0]);
    assert_eq!(slide(&board, Direction::Right).0[0], [0, 0, 0, 2]);
    let up = slide(&board, Direction::Up).0;
    assert_eq!((up[0][1], up[1][1], up[3][1]), (3, 0, 0));
    let down = slide(&board, Direction::Down).0;
    assert_eq!((down[3][0], down[3][1], down[3][3]), (1, 3, 1));
}

#[test]
fn a_move_adds_score_and_spawns_exactly_one_tile() {
    let board = [[1, 1, 0, 0], [0; 4], [0; 4], [0; 4]];
    let mut game = Game::from_board(board, 3);
    let outcome = game.apply(Direction::Left).unwrap();
    assert_eq!(outcome.gained, 4);
    assert_eq!(game.score(), 4);
    assert_eq!(game.board()[0][0], 2);
    assert_eq!(
        game.board().iter().flatten().filter(|&&t| t != 0).count(),
        2
    );
    let Spawn { row, col, exponent } = outcome.spawned;
    assert_eq!(game.board()[row][col], exponent);
    assert!(exponent == 1 || exponent == 2);
}

#[test]
fn unchanged_moves_and_full_boards_are_rejected() {
    let board = [[1, 0, 0, 0], [0; 4], [0; 4], [0; 4]];
    let mut game = Game::from_board(board, 0);
    assert_eq!(game.apply(Direction::Left), Err(MoveError::NoChange));
    assert_eq!(game.apply(Direction::Up), Err(MoveError::NoChange));
    assert!(!game.can_undo());
    let stuck = [[1, 2, 1, 2], [2, 1, 2, 1], [1, 2, 1, 2], [2, 1, 2, 1]];
    let mut game = Game::from_board(stuck, 0);
    assert_eq!(game.status(), Status::Over);
    assert_eq!(game.apply(Direction::Down), Err(MoveError::GameOver));
}

#[test]
fn reaching_2048_wins_but_play_continues() {
    let board = [[10, 10, 0, 0], [0; 4], [0; 4], [0; 4]];
    let mut game = Game::from_board(board, 0);
    let outcome = game.apply(Direction::Left).unwrap();
    assert_eq!(outcome.status, Status::Won);
    assert_eq!(game.max_exponent(), WIN_EXPONENT);
    assert!(game.apply(Direction::Right).is_ok());
}

#[test]
fn seeds_are_reproducible_including_after_undo() {
    let play = |seed| {
        let mut game = Game::new(seed);
        for direction in Direction::ALL.into_iter().cycle().take(40) {
            let _ = game.apply(direction);
        }
        (*game.board(), game.score())
    };
    assert_eq!(play(9), play(9));
    assert_eq!(Game::new(9).board(), Game::new(9).board());

    let mut game = Game::new(5);
    let direction = Direction::ALL
        .into_iter()
        .find(|&d| game.validate(d).is_ok())
        .unwrap();
    let first = game.apply(direction).unwrap();
    assert!(game.undo());
    assert_eq!(
        game.apply(direction).unwrap(),
        first,
        "undo restores the RNG"
    );
}

proptest! {
    #[test]
    fn tiles_are_conserved_and_history_round_trips(
        seed in any::<u64>(),
        moves in prop::collection::vec(0usize..4, 1..80),
    ) {
        let mut game = Game::new(seed);
        let total = |board: &Board| -> u64 {
            board.iter().flatten().filter(|&&t| t != 0).map(|&t| 1u64 << t).sum()
        };
        let mut states = vec![(*game.board(), game.score())];
        for index in moves {
            let before = total(game.board());
            match game.apply(Direction::ALL[index]) {
                Ok(outcome) => {
                    let spawned = 1u64 << outcome.spawned.exponent;
                    prop_assert_eq!(total(game.board()), before + spawned);
                    states.push((*game.board(), game.score()));
                }
                Err(_) => prop_assert_eq!(&(*game.board(), game.score()), states.last().unwrap()),
            }
        }
        for expected in states.iter().rev().skip(1) {
            prop_assert!(game.undo());
            prop_assert_eq!(&(*game.board(), game.score()), expected);
        }
        prop_assert!(!game.undo());
    }
}
