use crate::{game::State, *};
use proptest::prelude::*;
use std::collections::HashSet;

fn card(suit: Suit, rank: Rank) -> Card {
    Card::new(suit, rank)
}
fn up(card: Card) -> TableauCard {
    TableauCard {
        card,
        face_up: true,
    }
}
fn down(card: Card) -> TableauCard {
    TableauCard {
        card,
        face_up: false,
    }
}
fn movement(column: usize, index: usize, target: usize) -> Action {
    Action::Move {
        from: Source::Tableau { column, index },
        to: Target::Tableau(target),
    }
}

fn fixture(tableau: [Vec<TableauCard>; 7], waste: Vec<Card>, foundations: [Vec<Card>; 4]) -> Game {
    let used: HashSet<_> = tableau
        .iter()
        .flatten()
        .map(|c| c.card)
        .chain(waste.iter().copied())
        .chain(foundations.iter().flatten().copied())
        .collect();
    let stock = Suit::ALL
        .into_iter()
        .flat_map(|suit| Rank::ALL.into_iter().map(move |rank| card(suit, rank)))
        .filter(|c| !used.contains(c))
        .collect();
    Game::from_state(
        State {
            stock,
            waste,
            tableau,
            foundations,
            moves: 0,
        },
        Rules::default(),
    )
}

fn columns() -> [Vec<TableauCard>; 7] {
    std::array::from_fn(|_| Vec::new())
}
fn foundations() -> [Vec<Card>; 4] {
    std::array::from_fn(|_| Vec::new())
}

fn assert_invariants(game: &Game) {
    let state = &game.state;
    let all: Vec<_> = state
        .stock
        .iter()
        .chain(&state.waste)
        .copied()
        .chain(state.tableau.iter().flatten().map(|c| c.card))
        .chain(state.foundations.iter().flatten().copied())
        .collect();
    assert_eq!(all.len(), 52);
    assert_eq!(all.into_iter().collect::<HashSet<_>>().len(), 52);
    for pile in &state.tableau {
        if let Some(top) = pile.last() {
            assert!(top.face_up);
        }
        let first_up = pile.iter().position(|c| c.face_up).unwrap_or(pile.len());
        assert!(pile[first_up..].iter().all(|c| c.face_up));
        for pair in pile[first_up..].windows(2) {
            assert_ne!(pair[0].card.color(), pair[1].card.color());
            assert_eq!(pair[0].card.rank.value(), pair[1].card.rank.value() + 1);
        }
    }
    for (index, pile) in state.foundations.iter().enumerate() {
        for (rank, c) in pile.iter().enumerate() {
            assert_eq!(c.suit.index(), index);
            assert_eq!(c.rank.value() as usize, rank + 1);
        }
    }
}

#[test]
fn initial_deal_and_determinism() {
    let game = Game::new(42, Rules::default());
    assert_eq!(game.state.stock.len(), 24);
    for (column, pile) in game.state.tableau.iter().enumerate() {
        assert_eq!(pile.len(), column + 1);
        assert_eq!(pile.iter().filter(|c| c.face_up).count(), 1);
    }
    assert_invariants(&game);
    assert_eq!(game.state, Game::new(42, Rules::default()).state);
    assert_ne!(game.state, Game::new(43, Rules::default()).state);
    let tops: Vec<_> = game
        .state
        .tableau
        .iter()
        .map(|p| p.last().unwrap().card.to_string())
        .collect();
    assert_eq!(tops, ["QC", "8C", "8D", "KC", "2S", "5D", "2D"]);
}

#[test]
fn auto_finish_waits_for_hidden_cards() {
    for seed in 0..40 {
        assert!(
            Game::new(seed, Rules::default())
                .auto_finish_plan()
                .is_none()
        );
    }
    let mut piles = columns();
    piles[0] = vec![down(card(Suit::Clubs, Rank::King))];
    let complete = Suit::ALL.map(|suit| {
        Rank::ALL
            .into_iter()
            .filter(|rank| suit != Suit::Clubs || *rank != Rank::King)
            .map(|rank| card(suit, rank))
            .collect()
    });
    assert!(
        fixture(piles, vec![], complete)
            .auto_finish_plan()
            .is_none()
    );
}

#[test]
fn auto_finish_proves_every_move_without_changing_state_or_history() {
    let mut piles = columns();
    for (index, suit) in Suit::ALL.into_iter().enumerate() {
        piles[index].push(up(card(suit, Rank::King)));
    }
    let complete = Suit::ALL.map(|suit| {
        Rank::ALL[..12]
            .iter()
            .map(|rank| card(suit, *rank))
            .collect()
    });
    let mut game = fixture(piles, vec![], complete);
    let before = game.state.clone();
    let plan = game.auto_finish_plan().unwrap();
    assert_eq!(plan.len(), 4);
    assert_eq!(game.state, before);
    assert!(!game.can_undo());
    for action in &plan {
        game.apply(*action).unwrap();
    }
    assert_eq!(game.status(), Status::Won);
    assert!(game.auto_finish_plan().is_none());
    for _ in &plan {
        assert!(game.undo());
    }
    assert_eq!(game.state, before);
    assert!(game.can_redo());
    assert_eq!(game.auto_finish_plan(), Some(plan));
    assert!(game.can_redo());
}

#[test]
fn auto_finish_checks_draw_three_cycles_instead_of_assuming_face_up_means_won() {
    let complete = Suit::ALL.map(|suit| {
        Rank::ALL
            .into_iter()
            .filter(|rank| suit != Suit::Clubs || rank.value() < 10)
            .map(|rank| card(suit, rank))
            .collect()
    });
    let mut game = fixture(columns(), vec![], complete);
    game.state.stock = [Rank::King, Rank::Queen, Rank::Jack, Rank::Ten]
        .map(|rank| card(Suit::Clubs, rank))
        .to_vec();
    let game = Game::from_state(
        game.state,
        Rules {
            draw: DrawMode::Three,
        },
    );
    let before = game.state.clone();
    assert!(game.auto_finish_plan().is_none());
    assert_eq!(game.state, before);
    assert!(!game.can_undo());
    let mut game = Game::from_state(game.state, Rules::default());
    let plan = game.auto_finish_plan().unwrap();
    assert!(plan.contains(&Action::Draw));
    for action in plan {
        game.apply(action).unwrap();
    }
    assert_eq!(game.status(), Status::Won);
    assert_invariants(&game);
}

#[test]
fn sequence_move_flip_and_atomic_undo() {
    let mut piles = columns();
    piles[0] = vec![
        down(card(Suit::Clubs, Rank::Ace)),
        up(card(Suit::Hearts, Rank::Queen)),
        up(card(Suit::Spades, Rank::Jack)),
    ];
    piles[1] = vec![up(card(Suit::Clubs, Rank::King))];
    let mut game = fixture(piles, vec![], foundations());
    let initial = game.state.clone();
    game.apply(movement(0, 1, 1)).unwrap();
    assert_eq!(game.view().moves, 1);
    assert!(game.state.tableau[0][0].face_up);
    assert_eq!(game.state.tableau[1].len(), 3);
    let moved = game.state.clone();
    assert!(game.undo());
    assert_eq!(game.state, initial);
    assert!(game.redo());
    assert_eq!(game.state, moved);
    assert_invariants(&game);
}

#[test]
fn invalid_moves_leave_state_and_redo_untouched() {
    let mut game = Game::new(0, Rules::default());
    game.apply(Action::Draw).unwrap();
    game.undo();
    let initial = game.state.clone();
    let invalid = [
        movement(7, 0, 1),
        movement(1, usize::MAX, 0),
        movement(0, 0, 7),
        movement(1, 0, 0),
        movement(0, 0, 0),
        Action::Recycle,
        Action::Move {
            from: Source::Foundation(4),
            to: Target::Tableau(0),
        },
        Action::Move {
            from: Source::Waste,
            to: Target::Foundation(0),
        },
    ];
    for action in invalid {
        assert!(game.apply(action).is_err(), "{action:?}");
        assert_eq!(game.state, initial);
        assert!(game.can_redo());
        assert!(!game.can_undo());
    }
    game.apply(Action::Draw).unwrap();
    assert!(!game.can_redo());
}

#[test]
fn empty_columns_accept_only_king_sequences() {
    let mut piles = columns();
    piles[0] = vec![
        up(card(Suit::Clubs, Rank::King)),
        up(card(Suit::Hearts, Rank::Queen)),
    ];
    let mut game = fixture(piles, vec![], foundations());
    assert_eq!(game.apply(movement(0, 1, 1)), Err(MoveError::RequiresKing));
    game.apply(movement(0, 0, 1)).unwrap();
    assert_eq!(game.state.tableau[1].len(), 2);
    assert_invariants(&game);
}

#[test]
fn tableau_color_rank_and_sequence_are_enforced() {
    let mut piles = columns();
    piles[0] = vec![up(card(Suit::Hearts, Rank::Queen))];
    piles[1] = vec![up(card(Suit::Diamonds, Rank::King))];
    piles[2] = vec![up(card(Suit::Spades, Rank::Jack))];
    let mut game = fixture(piles, vec![], foundations());
    assert_eq!(
        game.apply(movement(0, 0, 1)),
        Err(MoveError::WrongColorOrRank)
    );
    assert_eq!(
        game.apply(movement(0, 0, 2)),
        Err(MoveError::WrongColorOrRank)
    );
    // A malformed sequence cannot be moved, even onto a compatible card.
    game.state.tableau[0].push(up(card(Suit::Hearts, Rank::Jack)));
    assert_eq!(
        game.validate(movement(0, 0, 3)),
        Err(MoveError::InvalidSequence)
    );
}

#[test]
fn foundations_match_suit_order_and_allow_return_to_tableau() {
    let mut piles = columns();
    piles[0] = vec![up(card(Suit::Hearts, Rank::Two))];
    let mut game = fixture(piles, vec![card(Suit::Clubs, Rank::Ace)], foundations());
    assert_eq!(
        game.apply(Action::Move {
            from: Source::Waste,
            to: Target::Foundation(1)
        }),
        Err(MoveError::WrongFoundation)
    );
    let initial = game.state.clone();
    let ace_up = Action::Move {
        from: Source::Waste,
        to: Target::Foundation(0),
    };
    game.apply(ace_up).unwrap();
    assert!(game.undo());
    assert_eq!(game.state, initial);
    game.redo();
    assert_eq!(
        game.validate(Action::Move {
            from: Source::Tableau {
                column: 0,
                index: 0
            },
            to: Target::Foundation(2)
        }),
        Err(MoveError::WrongFoundation)
    );
    game.apply(Action::Move {
        from: Source::Foundation(0),
        to: Target::Tableau(0),
    })
    .unwrap();
    assert_eq!(
        game.state.tableau[0].last().unwrap().card,
        card(Suit::Clubs, Rank::Ace)
    );
    assert_invariants(&game);
}

#[test]
fn foundations_reject_multiple_cards_and_skipped_ranks() {
    let mut piles = columns();
    piles[0] = vec![
        up(card(Suit::Hearts, Rank::Two)),
        up(card(Suit::Clubs, Rank::Ace)),
    ];
    let game = fixture(piles, vec![card(Suit::Diamonds, Rank::Two)], foundations());
    assert_eq!(
        game.validate(Action::Move {
            from: Source::Tableau {
                column: 0,
                index: 0
            },
            to: Target::Foundation(2)
        }),
        Err(MoveError::SingleCardRequired)
    );
    assert_eq!(
        game.validate(Action::Move {
            from: Source::Waste,
            to: Target::Foundation(1)
        }),
        Err(MoveError::WrongFoundation)
    );
}

#[test]
fn draw_modes_recycle_order_and_history() {
    for mode in [DrawMode::One, DrawMode::Three] {
        let mut game = Game::new(93, Rules { draw: mode });
        let initial = game.state.clone();
        let draw_order: Vec<_> = initial.stock.iter().rev().copied().collect();
        while !game.state.stock.is_empty() {
            game.apply(Action::Draw).unwrap();
        }
        assert_eq!(game.state.waste, draw_order);
        let exhausted = game.state.clone();
        assert_eq!(game.apply(Action::Draw), Err(MoveError::EmptyStock));
        game.apply(Action::Recycle).unwrap();
        assert_eq!(game.state.stock, initial.stock);
        assert!(game.state.waste.is_empty());
        game.undo();
        assert_eq!(game.state, exhausted);
        game.redo();
        game.apply(Action::Draw).unwrap();
        assert_eq!(game.state.waste, draw_order[..mode.count()]);
        assert_invariants(&game);
    }
}

#[test]
fn draw_three_handles_tail_and_exposes_only_waste_top() {
    let mut game = Game::new(
        4,
        Rules {
            draw: DrawMode::Three,
        },
    );
    // Isolate the two-card stock edge case; full-deck invariants are tested elsewhere.
    game.state.stock.truncate(2);
    let expected: Vec<_> = game.state.stock.iter().rev().copied().collect();
    game.apply(Action::Draw).unwrap();
    assert_eq!(game.state.waste, expected);
    assert!(game.state.stock.is_empty());
    let mut piles = columns();
    piles[0] = vec![up(card(Suit::Spades, Rank::Three))];
    let mut game = fixture(
        piles,
        vec![card(Suit::Clubs, Rank::Ace), card(Suit::Hearts, Rank::Two)],
        foundations(),
    );
    assert!(
        game.validate(Action::Move {
            from: Source::Waste,
            to: Target::Foundation(0)
        })
        .is_err()
    );
    game.apply(Action::Move {
        from: Source::Waste,
        to: Target::Tableau(0),
    })
    .unwrap();
    game.apply(Action::Move {
        from: Source::Waste,
        to: Target::Foundation(0),
    })
    .unwrap();
    assert_invariants(&game);
    game.state.stock.clear();
    game.state.waste.clear();
    assert_eq!(game.validate(Action::Recycle), Err(MoveError::EmptyWaste));
}

#[test]
fn win_and_undo_win() {
    let built = Suit::ALL.map(|suit| {
        Rank::ALL
            .into_iter()
            .filter(|rank| *rank != Rank::King)
            .map(|rank| card(suit, rank))
            .collect()
    });
    let mut game = fixture(
        columns(),
        Suit::ALL
            .into_iter()
            .rev()
            .map(|suit| card(suit, Rank::King))
            .collect(),
        built,
    );
    for pile in 0..4 {
        let result = game
            .apply(Action::Move {
                from: Source::Waste,
                to: Target::Foundation(pile),
            })
            .unwrap();
        assert_eq!(
            result,
            if pile == 3 {
                Status::Won
            } else {
                Status::Playing
            }
        );
    }
    assert_eq!(game.status(), Status::Won);
    assert!(game.legal_actions().is_empty());
    assert_eq!(game.apply(Action::Draw), Err(MoveError::GameWon));
    assert_invariants(&game);
    game.undo();
    assert_eq!(game.status(), Status::Playing);
    game.redo();
    assert_eq!(game.status(), Status::Won);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]
    #[test]
    fn legal_play_preserves_cards_and_roundtrips_history(seed in any::<u64>(), choices in prop::collection::vec(any::<usize>(), 1..160), three in any::<bool>()) {
        let mut game = Game::new(seed, Rules { draw: if three { DrawMode::Three } else { DrawMode::One } });
        let initial = game.state.clone();
        let mut count = 0;
        for choice in choices {
            assert_invariants(&game);
            let actions = game.legal_actions();
            if actions.is_empty() { break; }
            let before = game.state.clone();
            game.apply(actions[choice % actions.len()]).unwrap();
            let after = game.state.clone();
            prop_assert!(game.undo()); prop_assert_eq!(&game.state, &before);
            prop_assert!(game.redo()); prop_assert_eq!(&game.state, &after);
            count += 1;
        }
        let final_state = game.state.clone();
        assert_invariants(&game);
        for _ in 0..count { prop_assert!(game.undo()); }
        prop_assert_eq!(&game.state, &initial);
        prop_assert!(!game.undo());
        for _ in 0..count { prop_assert!(game.redo()); }
        prop_assert_eq!(&game.state, &final_state);
        prop_assert!(!game.redo());
    }

    #[test]
    fn enumeration_matches_validation(seed in any::<u64>(), index in 0usize..10) {
        let game = Game::new(seed, Rules::default());
        let legal = game.legal_actions();
        for column in 0..9 {
            for to in (0..9).map(Target::Tableau).chain((0..6).map(Target::Foundation)) {
                let action = Action::Move { from: Source::Tableau { column, index }, to };
                prop_assert_eq!(legal.contains(&action), game.validate(action).is_ok());
            }
        }
    }
}
