use crate::*;
use proptest::prelude::*;

const PUZZLE: &str = "
    53..7....
    6..195...
    .98....6.
    8...6...3
    4..8.3..1
    7...2...6
    .6....28.
    ...419..5
    ....8..79";

const SOLUTION: &str =
    "534678912672195348198342567859761423426853791713924856961537284287419635345286179";

fn at(row: usize, col: usize) -> Position {
    Position::new(row, col)
}

fn is_complete(grid: &Grid) -> bool {
    let unit_ok = |cells: [usize; 9]| {
        let mut seen = 0u16;
        cells.iter().all(|&i| {
            let bit = 1 << grid[i];
            grid[i] != 0 && seen & bit == 0 && {
                seen |= bit;
                true
            }
        })
    };
    (0..9).all(|n| {
        unit_ok(std::array::from_fn(|k| n * 9 + k))
            && unit_ok(std::array::from_fn(|k| k * 9 + n))
            && unit_ok(std::array::from_fn(|k| {
                (n / 3 * 3 + k / 3) * 9 + n % 3 * 3 + k % 3
            }))
    })
}

#[test]
fn solves_a_known_puzzle_uniquely() {
    let game = Game::from_puzzle(PUZZLE).unwrap();
    assert_eq!(game.givens_count(), 30);
    let expected: Vec<u8> = SOLUTION.bytes().map(|b| b - b'0').collect();
    let mut game = game;
    for (i, &digit) in expected.iter().enumerate() {
        let position = at(i / 9, i % 9);
        if !game.is_given(position) {
            game.apply(Action::Set {
                at: position,
                digit,
            })
            .unwrap();
        }
    }
    assert_eq!(game.status(), Status::Solved);
    assert_eq!(
        game.apply(Action::Clear { at: at(0, 2) }),
        Err(MoveError::Solved)
    );
}

#[test]
fn ambiguous_or_broken_puzzles_are_rejected() {
    assert!(Game::from_puzzle(&".".repeat(81)).is_none());
    let mut broken: String = PUZZLE.split_whitespace().collect();
    broken.replace_range(2..3, "5");
    assert!(Game::from_puzzle(&broken).is_none());
    assert!(solutions(&[0; CELLS], 3).len() == 3);
}

#[test]
fn givens_are_fixed_and_invalid_input_is_ignored() {
    let mut game = Game::from_puzzle(PUZZLE).unwrap();
    let cases = [
        (
            Action::Set {
                at: at(0, 0),
                digit: 1,
            },
            MoveError::GivenCell,
        ),
        (
            Action::Set {
                at: at(0, 2),
                digit: 0,
            },
            MoveError::InvalidDigit,
        ),
        (
            Action::Set {
                at: at(9, 0),
                digit: 1,
            },
            MoveError::OutOfRange,
        ),
        (Action::Clear { at: at(0, 2) }, MoveError::Unchanged),
    ];
    for (action, error) in cases {
        assert_eq!(game.apply(action), Err(error));
    }
    assert!(!game.can_undo());
    assert_eq!(game.moves(), 0);
    game.apply(Action::Set {
        at: at(0, 2),
        digit: 4,
    })
    .unwrap();
    assert_eq!(
        game.apply(Action::Set {
            at: at(0, 2),
            digit: 4
        }),
        Err(MoveError::Unchanged)
    );
    assert_eq!(
        game.apply(Action::ToggleNote {
            at: at(0, 2),
            digit: 1
        }),
        Err(MoveError::CellFilled)
    );
}

#[test]
fn conflicts_and_mistakes_are_reported_separately() {
    let mut game = Game::from_puzzle(PUZZLE).unwrap();
    // 5 already appears in row 0: a rule conflict and a mistake.
    game.apply(Action::Set {
        at: at(0, 2),
        digit: 5,
    })
    .unwrap();
    assert!(game.conflicts(at(0, 2)));
    assert!(game.conflicts(at(0, 0)));
    // 2 breaks no rule yet but differs from the solution (4).
    game.apply(Action::Set {
        at: at(0, 2),
        digit: 2,
    })
    .unwrap();
    assert!(!game.conflicts(at(0, 2)));
    assert_eq!(game.mistakes(), vec![at(0, 2)]);
}

#[test]
fn placing_a_digit_prunes_peer_notes_and_undo_restores_them() {
    let mut game = Game::from_puzzle(PUZZLE).unwrap();
    for position in [at(0, 3), at(1, 2), at(4, 2)] {
        game.apply(Action::ToggleNote {
            at: position,
            digit: 4,
        })
        .unwrap();
    }
    game.apply(Action::ToggleNote {
        at: at(0, 2),
        digit: 9,
    })
    .unwrap();
    game.apply(Action::Set {
        at: at(0, 2),
        digit: 4,
    })
    .unwrap();
    assert!(!game.has_note(at(0, 2), 9));
    assert!(!game.has_note(at(0, 3), 4), "same row");
    assert!(!game.has_note(at(1, 2), 4), "same box");
    assert!(!game.has_note(at(4, 2), 4), "same column");
    assert!(game.undo());
    assert!(game.has_note(at(0, 3), 4) && game.has_note(at(0, 2), 9));
    assert!(game.redo());
    assert_eq!(game.value(at(0, 2)), Some(4));
}

#[test]
fn hints_follow_the_solution() {
    let mut game = Game::from_puzzle(PUZZLE).unwrap();
    while let Some(hint) = game.hint() {
        assert_eq!(game.value(hint.at), None);
        game.apply(Action::Set {
            at: hint.at,
            digit: hint.digit,
        })
        .unwrap();
    }
    assert_eq!(game.status(), Status::Solved);
    assert!(game.mistakes().is_empty());
}

#[test]
fn generation_is_deterministic_and_respects_difficulty() {
    for difficulty in Difficulty::ALL {
        let a = Game::new(7, difficulty);
        let b = Game::new(7, difficulty);
        let grid = |game: &Game| -> Vec<Option<u8>> {
            (0..CELLS).map(|i| game.value(at(i / 9, i % 9))).collect()
        };
        assert_eq!(grid(&a), grid(&b));
        assert!(a.givens_count() >= difficulty.target_givens());
        assert!(a.givens_count() <= difficulty.target_givens() + 6);
    }
    let first = Game::new(1, Difficulty::Medium);
    let second = Game::new(2, Difficulty::Medium);
    assert_ne!(
        (0..CELLS)
            .map(|i| first.value(at(i / 9, i % 9)))
            .collect::<Vec<_>>(),
        (0..CELLS)
            .map(|i| second.value(at(i / 9, i % 9)))
            .collect::<Vec<_>>()
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn generated_puzzles_are_unique_and_solvable(seed in any::<u64>()) {
        let game = Game::new(seed, Difficulty::Hard);
        let givens: Grid = std::array::from_fn(|i| game.value(at(i / 9, i % 9)).unwrap_or(0));
        let found = solutions(&givens, 2);
        prop_assert_eq!(found.len(), 1);
        prop_assert!(is_complete(&found[0]));
        for i in 0..CELLS {
            prop_assert!(givens[i] == 0 || givens[i] == found[0][i]);
        }
    }

    #[test]
    fn undo_and_redo_round_trip(seed in any::<u64>(), actions in prop::collection::vec((0usize..81, 0u8..=9, 0u8..3), 1..40)) {
        let mut game = Game::new(seed, Difficulty::Easy);
        let snapshot = |game: &Game| -> Vec<(Option<u8>, u16)> {
            (0..CELLS).map(|i| {
                let p = at(i / 9, i % 9);
                (game.value(p), (1..=9).filter(|&d| game.has_note(p, d)).fold(0, |m, d| m | 1 << d))
            }).collect()
        };
        let mut states = vec![snapshot(&game)];
        for (cell, digit, kind) in actions {
            let at = at(cell / 9, cell % 9);
            let action = match kind {
                0 => Action::Set { at, digit },
                1 => Action::Clear { at },
                _ => Action::ToggleNote { at, digit },
            };
            let before = snapshot(&game);
            if game.apply(action).is_ok() {
                states.push(snapshot(&game));
            } else {
                prop_assert_eq!(snapshot(&game), before);
            }
        }
        for expected in states.iter().rev().skip(1) {
            prop_assert!(game.undo());
            prop_assert_eq!(&snapshot(&game), expected);
        }
        prop_assert!(!game.undo());
        for expected in states.iter().skip(1) {
            prop_assert!(game.redo());
            prop_assert_eq!(&snapshot(&game), expected);
        }
    }
}
