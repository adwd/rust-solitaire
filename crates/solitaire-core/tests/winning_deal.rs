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

#[test]
fn ordinary_deal_can_finish_early_and_preserve_the_entire_history() {
    let mut game = Game::new(6, Rules::default());
    let initial = format!("{:?}", game.view());
    let mut manual_moves = 0;
    let mut plan = None;
    for line in include_str!("fixtures/seed-6.moves").lines() {
        game.apply(parse(line)).unwrap();
        manual_moves += 1;
        if let Some(finish) = game.auto_finish_plan() {
            plan = Some(finish);
            break;
        }
    }
    let plan = plan.expect("a real deal becomes provably finishable before the last move");
    assert!(manual_moves < 238);
    assert!(
        game.view()
            .tableau
            .iter()
            .flatten()
            .all(|card| card.face_up)
    );
    for action in &plan {
        assert!(!matches!(
            action,
            Action::Move {
                to: Target::Tableau(_),
                ..
            }
        ));
        game.apply(*action).unwrap();
    }
    assert_eq!(game.status(), Status::Won);
    let final_state = format!("{:?}", game.view());
    for _ in 0..manual_moves + plan.len() {
        assert!(game.undo());
    }
    assert_eq!(format!("{:?}", game.view()), initial);
    for _ in 0..manual_moves + plan.len() {
        assert!(game.redo());
    }
    assert_eq!(format!("{:?}", game.view()), final_state);
    eprintln!(
        "Auto finish starts after {manual_moves} manual moves; {} automatic actions",
        plan.len()
    );
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
