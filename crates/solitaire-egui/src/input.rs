use eframe::egui::{Pos2, Vec2};
use solitaire_core::{Action, Game, Source, Target};

#[derive(Debug, Clone, Copy)]
pub struct Drag {
    pub from: Source,
    pub offset: Vec2,
    pub origin: Pos2,
}

#[derive(Debug, Default)]
pub struct Interaction {
    pub selected: Option<Source>,
    pub drag: Option<Drag>,
    pub pressed: Option<Drag>,
}

impl Interaction {
    pub fn clear(&mut self) {
        self.selected = None;
        self.drag = None;
        self.pressed = None;
    }

    pub fn select(&mut self, from: Option<Source>) {
        self.selected = from.filter(|from| Some(*from) != self.selected);
    }

    pub fn start_drag(&mut self, from: Source, origin: Pos2, card_min: Pos2) {
        self.pressed = None;
        self.selected = Some(from);
        self.drag = Some(Drag {
            from,
            offset: card_min - origin,
            origin,
        });
    }
}

pub fn foundation_move(game: &Game, from: Source) -> Option<Action> {
    game.legal_actions().into_iter().find(|action| {
        matches!(action,
        Action::Move { from: source, to: Target::Foundation(_) } if *source == from)
    })
}

pub fn source_selected(selected: Option<Source>, source: Source) -> bool {
    match (selected, source) {
        (
            Some(Source::Tableau {
                column: a,
                index: start,
            }),
            Source::Tableau { column: b, index },
        ) => a == b && index >= start,
        (Some(selected), source) => selected == source,
        _ => false,
    }
}
