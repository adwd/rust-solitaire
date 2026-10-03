use solitaire_core::{Action, Game, Rules, Source, Status, Target};

// A complete legal game from the ordinary seeded deal, not a synthetic near-win state.
#[test]
fn seed_six_wins_and_full_history_roundtrips() {
    let mut game = Game::new(6, Rules::default());
    let initial = format!("{:?}", game.view());
    let mut count = 0;
    for line in include_str!("fixtures/seed-6.moves").lines() {
        game.apply(parse(line)).unwrap();
        count += 1;
    }
    assert_eq!(count, 238);
    assert_eq!(game.status(), Status::Won);
    assert!(game.view().foundations.iter().all(|pile| pile.len() == 13));
    let complete = format!("{:?}", game.view());
    for _ in 0..count {
        assert!(game.undo());
    }
    assert_eq!(format!("{:?}", game.view()), initial);
    for _ in 0..count {
        assert!(game.redo());
    }
    assert_eq!(format!("{:?}", game.view()), complete);
}

fn parse(line: &str) -> Action {
    match line {
        "D" => Action::Draw,
        "R" => Action::Recycle,
        _ => {
            let (source, target) = line.split_once('>').unwrap();
            let from = if source == "W" {
                Source::Waste
            } else {
                let (column, index) = source[1..].split_once(':').unwrap();
                Source::Tableau {
                    column: column.parse().unwrap(),
                    index: index.parse().unwrap(),
                }
            };
            let index = target[1..].parse().unwrap();
            let to = if target.starts_with('F') {
                Target::Foundation(index)
            } else {
                Target::Tableau(index)
            };
            Action::Move { from, to }
        }
    }
}
