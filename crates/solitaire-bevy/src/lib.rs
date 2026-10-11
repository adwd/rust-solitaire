//! Presentation state and table geometry. All rules and history live in solitaire-core.
use bevy::prelude::*;
use solitaire_core::{Action, Card, DrawMode, Game, Rules, Source, Status, Target};
use std::collections::VecDeque;

pub const CARD_W: f32 = 1.62;
pub const CARD_H: f32 = 2.38;
pub const TOP_Z: f32 = -3.7;
pub const TABLEAU_Z: f32 = -0.5;

pub fn card_id(card: Card) -> usize {
    card.suit.index() * 13 + card.rank.value() as usize - 1
}

pub fn column_x(column: usize) -> f32 {
    (column as f32 - 3.0) * 1.9
}

#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub card: Card,
    pub position: Vec3,
    pub face_up: bool,
    pub source: Option<Source>,
}

pub fn layout(game: &Game) -> Vec<Placement> {
    let view = game.view();
    let mut out = Vec::with_capacity(52);
    for (i, &card) in view.stock.iter().enumerate() {
        out.push(Placement {
            card,
            position: Vec3::new(column_x(0), 0.11 + i as f32 * 0.007, TOP_Z),
            face_up: false,
            source: None,
        });
    }
    for (i, &card) in view.waste.iter().enumerate() {
        let exposed = i.saturating_sub(view.waste.len().saturating_sub(3));
        out.push(Placement {
            card,
            position: Vec3::new(
                column_x(1) + exposed as f32 * 0.17,
                0.11 + i as f32 * 0.008,
                TOP_Z,
            ),
            face_up: true,
            source: (i + 1 == view.waste.len()).then_some(Source::Waste),
        });
    }
    for (pile, cards) in view.foundations.iter().enumerate() {
        for (i, &card) in cards.iter().enumerate() {
            out.push(Placement {
                card,
                position: Vec3::new(column_x(pile + 3), 0.11 + i as f32 * 0.012, TOP_Z),
                face_up: true,
                source: (i + 1 == cards.len()).then_some(Source::Foundation(pile)),
            });
        }
    }
    for (column, cards) in view.tableau.iter().enumerate() {
        let hidden = cards.iter().filter(|c| !c.face_up).count();
        let exposed = cards.len() - hidden;
        let step = ((4.9 - hidden as f32 * 0.14) / exposed.saturating_sub(1).max(1) as f32)
            .clamp(0.38, 0.52);
        let mut z = TABLEAU_Z;
        for (index, placed) in cards.iter().enumerate() {
            out.push(Placement {
                card: placed.card,
                position: Vec3::new(column_x(column), 0.11 + index as f32 * 0.022, z),
                face_up: placed.face_up,
                source: placed.face_up.then_some(Source::Tableau { column, index }),
            });
            z += if placed.face_up { step } else { 0.14 };
        }
    }
    out
}

pub fn target_position(game: &Game, target: Target) -> Vec3 {
    match target {
        Target::Foundation(pile) => Vec3::new(column_x(pile + 3), 0.08, TOP_Z),
        Target::Tableau(column) => layout(game)
            .iter()
            .rev()
            .find(|p| matches!(p.source, Some(Source::Tableau { column: c, .. }) if c == column))
            .map_or(Vec3::new(column_x(column), 0.08, TABLEAU_Z), |p| p.position),
    }
}

pub fn source_card(game: &Game, source: Source) -> Option<Card> {
    let view = game.view();
    match source {
        Source::Waste => view.waste.last().copied(),
        Source::Foundation(pile) => view.foundations.get(pile)?.last().copied(),
        Source::Tableau { column, index } => view
            .tableau
            .get(column)?
            .get(index)
            .filter(|p| p.face_up)
            .map(|p| p.card),
    }
}

pub fn selected_cards(game: &Game, source: Source) -> Vec<Card> {
    match source {
        Source::Tableau { column, index } => game
            .view()
            .tableau
            .get(column)
            .and_then(|p| p.get(index..))
            .map_or_else(Vec::new, |p| p.iter().map(|c| c.card).collect()),
        _ => source_card(game, source).into_iter().collect(),
    }
}

#[derive(Resource)]
pub struct Table {
    pub game: Game,
    pub selection: Option<Source>,
    pub hint: Option<Action>,
    pub message: String,
    pub elapsed: f64,
    pub started: bool,
    pub auto: VecDeque<Action>,
    pub auto_clock: f32,
    pub hint_index: usize,
    pub top_view: bool,
}

impl Table {
    pub fn new(seed: u64, draw: DrawMode) -> Self {
        Self {
            game: Game::new(seed, Rules { draw }),
            selection: None,
            hint: None,
            message: "Your table, your pace. Select a card or draw from the stock.".into(),
            elapsed: 0.0,
            started: false,
            auto: VecDeque::new(),
            auto_clock: 0.0,
            hint_index: 0,
            top_view: false,
        }
    }

    pub fn apply(&mut self, action: Action) -> bool {
        match self.game.apply(action) {
            Ok(status) => {
                self.started = true;
                self.selection = None;
                self.hint = None;
                self.hint_index = 0;
                self.message = if status == Status::Won {
                    "Beautifully played. All 52 cards are home.".into()
                } else {
                    "Drag a sequence, or select a card and click its destination.".into()
                };
                true
            }
            Err(error) => {
                self.message = error.to_string();
                false
            }
        }
    }

    pub fn draw(&mut self) -> bool {
        self.auto.clear();
        self.apply(if self.game.view().stock.is_empty() {
            Action::Recycle
        } else {
            Action::Draw
        })
    }

    pub fn promote(&mut self, source: Source) -> bool {
        self.auto.clear();
        if let Some(action) = (0..4)
            .map(|pile| Action::Move {
                from: source,
                to: Target::Foundation(pile),
            })
            .find(|&a| self.game.validate(a).is_ok())
        {
            self.apply(action)
        } else {
            self.message = "That card cannot go to a foundation yet.".into();
            false
        }
    }

    pub fn undo(&mut self) {
        self.cancel();
        self.message = if self.game.undo() {
            "Move undone."
        } else {
            "No moves to undo."
        }
        .into();
    }

    pub fn redo(&mut self) {
        self.cancel();
        self.message = if self.game.redo() {
            "Move restored."
        } else {
            "No moves to redo."
        }
        .into();
    }

    pub fn cancel(&mut self) {
        self.selection = None;
        self.hint = None;
        self.auto.clear();
        self.hint_index = 0;
    }

    pub fn show_hint(&mut self) {
        self.selection = None;
        self.auto.clear();
        let actions = self.game.legal_actions();
        if actions.is_empty() {
            self.hint = None;
            self.message = "No legal moves. Undo a move or try a new deal.".into();
            return;
        }
        let index = self.hint_index % actions.len();
        let action = actions[index];
        self.hint_index += 1;
        self.hint = Some(action);
        self.message = format!(
            "Hint {}/{}: {}",
            index + 1,
            actions.len(),
            describe_action(&self.game, action)
        );
    }

    pub fn finish(&mut self) {
        self.cancel();
        if let Some(plan) = self.game.auto_finish_plan() {
            self.auto = plan.into();
            self.auto_clock = 0.0;
            self.message = "Finishing the table. Undo or Esc stops automatic play.".into();
        } else {
            self.message =
                "Finish becomes available when the core can prove a complete path home.".into();
        }
    }
}

pub fn describe_action(game: &Game, action: Action) -> String {
    match action {
        Action::Draw => "draw from the stock".into(),
        Action::Recycle => "recycle the waste into the stock".into(),
        Action::Move { from, to } => {
            let card = source_card(game, from).map_or_else(|| "card".into(), |c| c.to_string());
            match to {
                Target::Foundation(_) => format!("move {card} to its foundation"),
                Target::Tableau(column) => format!("move {card} to column {}", column + 1),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_covers_every_card_after_draw_recycle_and_history() {
        for draw in [DrawMode::One, DrawMode::Three] {
            let mut table = Table::new(42, draw);
            for _ in 0..40 {
                table.draw();
                let placements = layout(&table.game);
                let mut ids: Vec<_> = placements.iter().map(|p| card_id(p.card)).collect();
                ids.sort();
                assert_eq!(ids, (0..52).collect::<Vec<_>>());
                assert!(placements.iter().all(|p| p.position.is_finite()));
                let waste = placements
                    .iter()
                    .filter(|p| p.source == Some(Source::Waste))
                    .count();
                assert_eq!(waste, usize::from(!table.game.view().waste.is_empty()));
            }
            for _ in 0..40 {
                table.undo();
            }
            assert_eq!(table.game.view().moves, 0);
            table.redo();
            assert_eq!(table.game.view().moves, 1);
        }
    }

    #[test]
    fn frontend_actions_match_the_shared_engine_and_invalid_input_is_atomic() {
        let mut table = Table::new(2026, DrawMode::Three);
        let mut reference = Game::new(
            2026,
            Rules {
                draw: DrawMode::Three,
            },
        );
        for _ in 0..80 {
            let actions = reference.legal_actions();
            let Some(&action) = actions.first() else {
                break;
            };
            reference.apply(action).unwrap();
            assert!(table.apply(action));
            assert_eq!(table.game.view().stock, reference.view().stock);
            assert_eq!(table.game.view().waste, reference.view().waste);
            assert_eq!(table.game.view().tableau, reference.view().tableau);
            assert_eq!(table.game.view().foundations, reference.view().foundations);
        }
        let before = table.game.view().moves;
        assert!(!table.apply(Action::Move {
            from: Source::Foundation(99),
            to: Target::Tableau(0)
        }));
        assert_eq!(table.game.view().moves, before);
    }

    #[test]
    fn hints_are_legal_and_do_not_change_the_game() {
        let mut table = Table::new(42, DrawMode::One);
        for _ in 0..50 {
            table.show_hint();
            if let Some(action) = table.hint {
                assert!(table.game.validate(action).is_ok());
            }
        }
        assert_eq!(table.game.view().moves, 0);
        table.finish();
        assert!(table.auto.is_empty());
        assert_eq!(table.game.view().moves, 0);
    }
}
