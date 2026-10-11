use crate::*;
use proptest::prelude::*;

fn at(row: usize, col: usize) -> Position {
    Position::new(row, col)
}

fn mine_positions(game: &Game) -> Vec<Position> {
    let config = game.config();
    (0..config.rows)
        .flat_map(|row| (0..config.cols).map(move |col| at(row, col)))
        .filter(|&position| game.has_mine(position))
        .collect()
}

#[test]
fn presets_and_custom_sizes_are_validated() {
    assert_eq!(Config::EXPERT, Config::custom(16, 30, 99).unwrap());
    assert_eq!(Config::custom(1, 1, 0), Err(ConfigError::TooSmall));
    assert_eq!(Config::custom(3, 3, 9), Err(ConfigError::TooManyMines));
    assert_eq!(Config::custom(100, 3, 9), Err(ConfigError::TooLarge));
}

#[test]
fn the_first_reveal_and_its_neighbors_are_safe() {
    for seed in 0..50 {
        let mut game = Game::new(seed, Config::EXPERT);
        assert_eq!(game.status(), Status::Ready);
        game.apply(Action::Reveal(at(0, 0))).unwrap();
        assert_eq!(game.cell(at(0, 0)), CellView::Revealed(0));
        assert_eq!(mine_positions(&game).len(), 99);
    }
}

#[test]
fn crowded_boards_keep_only_the_first_cell_safe() {
    let mut game = Game::new(1, Config::custom(3, 3, 8).unwrap());
    assert_eq!(game.apply(Action::Reveal(at(1, 1))), Ok(Status::Won));
    assert_eq!(game.cell(at(1, 1)), CellView::Revealed(8));
    assert_eq!(
        game.cell(at(0, 0)),
        CellView::Flagged,
        "won boards flag mines"
    );
}

#[test]
fn placement_depends_only_on_seed_and_first_click() {
    let mines = |seed| {
        let mut game = Game::new(seed, Config::INTERMEDIATE);
        game.apply(Action::Reveal(at(8, 8))).unwrap();
        mine_positions(&game)
    };
    assert_eq!(mines(3), mines(3));
    assert_ne!(mines(3), mines(4));
}

#[test]
fn zeros_flood_fill_and_stop_at_numbers_and_flags() {
    // . . . . .
    // . . . . .
    // . . . 1 1
    // . . . 1 *
    let config = Config::custom(4, 5, 1).unwrap();
    let mut game = Game::with_mines(config, &[at(3, 4)]);
    game.apply(Action::ToggleFlag(at(0, 4))).unwrap();
    assert_eq!(game.apply(Action::Reveal(at(0, 0))), Ok(Status::Playing));
    assert_eq!(game.cell(at(2, 3)), CellView::Revealed(1));
    assert_eq!(game.cell(at(0, 4)), CellView::Flagged);
    assert_eq!(game.cell(at(3, 4)), CellView::Hidden);
    game.apply(Action::ToggleFlag(at(0, 4))).unwrap();
    assert_eq!(game.apply(Action::Reveal(at(0, 4))), Ok(Status::Won));
}

#[test]
fn chords_reveal_neighbors_only_when_flags_match() {
    // * 1 .
    // 1 1 .
    // . . .
    let config = Config::custom(3, 3, 1).unwrap();
    let mut game = Game::with_mines(config, &[at(0, 0)]);
    game.apply(Action::Reveal(at(1, 1))).unwrap();
    assert_eq!(
        game.apply(Action::Chord(at(1, 1))),
        Err(MoveError::FlagCountMismatch)
    );
    assert_eq!(game.apply(Action::Chord(at(2, 2))), Err(MoveError::Hidden));
    game.apply(Action::ToggleFlag(at(0, 0))).unwrap();
    assert_eq!(game.mines_left(), 0);
    assert_eq!(game.apply(Action::Chord(at(1, 1))), Ok(Status::Won));
}

#[test]
fn a_wrong_chord_explodes_and_reveals_the_board() {
    let config = Config::custom(3, 3, 1).unwrap();
    let mut game = Game::with_mines(config, &[at(0, 0)]);
    game.apply(Action::Reveal(at(1, 1))).unwrap();
    game.apply(Action::ToggleFlag(at(0, 1))).unwrap();
    assert_eq!(game.apply(Action::Chord(at(1, 1))), Ok(Status::Lost));
    assert_eq!(game.cell(at(0, 0)), CellView::Exploded);
    assert_eq!(game.cell(at(0, 1)), CellView::WrongFlag);
    assert_eq!(
        game.apply(Action::Reveal(at(2, 2))),
        Err(MoveError::GameOver)
    );
}

#[test]
fn invalid_actions_do_not_change_the_board() {
    let mut game = Game::new(0, Config::BEGINNER);
    assert_eq!(
        game.apply(Action::Reveal(at(9, 0))),
        Err(MoveError::OutOfRange)
    );
    game.apply(Action::ToggleFlag(at(0, 0))).unwrap();
    assert_eq!(
        game.apply(Action::Reveal(at(0, 0))),
        Err(MoveError::Flagged)
    );
    assert_eq!(game.status(), Status::Ready);
    assert_eq!(game.moves(), 1);
    game.apply(Action::Reveal(at(4, 4))).unwrap();
    assert_eq!(
        game.apply(Action::ToggleFlag(at(4, 4))),
        Err(MoveError::Revealed)
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn numbers_match_hidden_mines(seed in any::<u64>(), row in 0usize..16, col in 0usize..30) {
        let mut game = Game::new(seed, Config::EXPERT);
        prop_assert_ne!(game.apply(Action::Reveal(at(row, col))), Ok(Status::Lost));
        let mines = mine_positions(&game);
        prop_assert_eq!(mines.len(), 99);
        for r in 0..16 {
            for c in 0..30 {
                if let CellView::Revealed(n) = game.cell(at(r, c)) {
                    let expected = mines
                        .iter()
                        .filter(|m| m.row.abs_diff(r) <= 1 && m.col.abs_diff(c) <= 1)
                        .count();
                    prop_assert_eq!(usize::from(n), expected);
                }
            }
        }
    }
}
