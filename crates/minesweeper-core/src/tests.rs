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

mod hints {
    use super::*;

    fn reveal(game: &mut Game, cells: &[Position]) {
        for &cell in cells {
            game.apply(Action::Reveal(cell)).unwrap();
        }
    }

    #[test]
    fn the_first_click_needs_no_deduction() {
        let game = Game::new(1, Config::BEGINNER);
        assert_eq!(game.hint(), Some(Hint::FirstReveal));
    }

    #[test]
    fn a_number_with_one_hidden_neighbor_points_to_its_mine() {
        // 1 * ?
        let mut game = Game::with_mines(Config::custom(1, 3, 1).unwrap(), &[at(0, 1)]);
        reveal(&mut game, &[at(0, 0)]);
        assert_eq!(
            game.hint(),
            Some(Hint::Mine {
                at: at(0, 1),
                clues: vec![at(0, 0)],
                mine_count: false,
            })
        );
    }

    #[test]
    fn chained_deductions_cite_every_number_they_used() {
        // 1 * 1 ?   the left 1 proves the mine, which satisfies the
        //           right 1 and clears the last cell
        let mut game = Game::with_mines(Config::custom(1, 4, 1).unwrap(), &[at(0, 1)]);
        reveal(&mut game, &[at(0, 0), at(0, 2)]);
        assert_eq!(
            game.hint(),
            Some(Hint::Safe {
                at: at(0, 3),
                clues: vec![at(0, 0), at(0, 2)],
                mine_count: false,
            })
        );
    }

    #[test]
    fn nested_numbers_reveal_the_one_two_one_pattern() {
        // . . . . .      hidden row with mines over the 1s
        // 1 2 1 . .      only the middle cell above the 2 is safe
        // 0 0 0 . .
        let config = Config::custom(3, 3, 2).unwrap();
        let mut game = Game::with_mines(config, &[at(0, 0), at(0, 2)]);
        reveal(&mut game, &[at(2, 1)]);
        assert_eq!(game.cell(at(1, 1)), CellView::Revealed(2));
        let Some(Hint::Safe { at: safe, .. }) = game.hint() else {
            panic!("expected a safe cell, got {:?}", game.hint());
        };
        assert_eq!(safe, at(0, 1));
    }

    #[test]
    fn a_flag_on_a_safe_cell_is_called_out() {
        // 1 * ?   once the mine is known, the mine count clears the last cell
        let mut game = Game::with_mines(Config::custom(1, 3, 1).unwrap(), &[at(0, 1)]);
        reveal(&mut game, &[at(0, 0)]);
        game.apply(Action::ToggleFlag(at(0, 1))).unwrap();
        assert!(matches!(game.hint(), Some(Hint::Safe { at: s, .. }) if s == at(0, 2)));
        // Flags are not evidence: a wrong flag is reported as a safe cell.
        game.apply(Action::ToggleFlag(at(0, 2))).unwrap();
        assert!(matches!(game.hint(), Some(Hint::Safe { at: s, .. }) if s == at(0, 2)));
    }

    #[test]
    fn an_even_split_leaves_only_a_guess() {
        // 0 1 ?
        // 0 1 ?   one mine in either right cell: nothing tells them apart.
        let mut game = Game::with_mines(Config::custom(2, 3, 1).unwrap(), &[at(0, 2)]);
        reveal(&mut game, &[at(0, 0)]);
        assert_eq!(game.hint(), Some(Hint::Guess));
    }

    #[test]
    fn the_mine_count_settles_cells_away_from_numbers() {
        // 0 1 ? ?    the 1s already account for the only mine,
        // 0 1 ? ?    so the far column is safe.
        let config = Config::custom(2, 4, 1).unwrap();
        let mut game = Game::with_mines(config, &[at(0, 2)]);
        reveal(&mut game, &[at(0, 0)]);
        assert!(matches!(
            game.hint(),
            Some(Hint::Safe { at, mine_count: true, .. }) if at.col == 3
        ));
    }

    #[test]
    fn no_hint_after_the_game_ends() {
        let config = Config::custom(2, 2, 1).unwrap();
        let mut game = Game::with_mines(config, &[at(0, 1)]);
        reveal(&mut game, &[at(0, 1)]);
        assert_eq!(game.hint(), None);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(48))]

        /// Following hints never loses: safe cells hold no mine and mine
        /// hints are real. A guess is the only way the game can stop.
        #[test]
        fn hints_are_always_correct(seed in any::<u64>(), preset in 0usize..3, first in 0usize..480) {
            let config = [Config::BEGINNER, Config::INTERMEDIATE, Config::EXPERT][preset];
            let mut game = Game::new(seed, config);
            let first = first % (config.rows * config.cols);
            game.apply(Action::Reveal(at(first / config.cols, first % config.cols))).unwrap();
            for _ in 0..config.rows * config.cols * 2 {
                match game.hint() {
                    Some(Hint::Safe { at, .. }) => {
                        prop_assert!(!game.has_mine(at), "safe hint on a mine at {:?}", at);
                        if game.cell(at) == CellView::Flagged {
                            game.apply(Action::ToggleFlag(at)).unwrap();
                        }
                        prop_assert_ne!(game.apply(Action::Reveal(at)).unwrap(), Status::Lost);
                    }
                    Some(Hint::Mine { at, .. }) => {
                        prop_assert!(game.has_mine(at), "mine hint on a safe cell at {:?}", at);
                        game.apply(Action::ToggleFlag(at)).unwrap();
                    }
                    Some(Hint::Guess) | None => break,
                    Some(Hint::FirstReveal) => prop_assert!(false, "already revealed"),
                }
            }
            prop_assert_ne!(game.status(), Status::Lost);
        }
    }
}
