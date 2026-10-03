use std::collections::VecDeque;

use eframe::egui::{self, Color32, Rect, Stroke, StrokeKind, Vec2};
use solitaire_core::{Action, Card, Game, Source, Target};

use crate::{board::BoardLayout, theme};

const LAUNCH_INTERVAL: f64 = 0.055;
const FLIGHT_DURATION: f64 = 0.32;

struct Flight {
    card: Card,
    from: Rect,
    foundation: usize,
    launched: f64,
}

pub(crate) struct FinishAnimation {
    actions: VecDeque<Action>,
    flights: Vec<Flight>,
    next_launch: f64,
}

impl FinishAnimation {
    pub fn new(plan: Vec<Action>, now: f64) -> Self {
        Self {
            actions: plan.into(),
            flights: Vec::new(),
            next_launch: now + 0.18,
        }
    }

    /// Commit one proven action per frame at most. A suspended tab resumes
    /// smoothly instead of catching up with a whole invisible batch of moves.
    pub fn advance(
        &mut self,
        game: &mut Game,
        layout: BoardLayout,
        now: f64,
    ) -> Result<bool, solitaire_core::MoveError> {
        self.flights
            .retain(|flight| now - flight.launched < FLIGHT_DURATION);
        if now >= self.next_launch
            && let Some(action) = self.actions.pop_front()
        {
            let flight = match action {
                Action::Move {
                    from,
                    to: Target::Foundation(foundation),
                } => {
                    let view = game.view();
                    let card = match from {
                        Source::Waste => *view.waste.last().expect("proven waste move"),
                        Source::Tableau { column, index } => view.tableau[column][index].card,
                        Source::Foundation(_) => unreachable!("proof only collects cards"),
                    };
                    Some(Flight {
                        card,
                        from: layout.source(view, from),
                        foundation,
                        launched: now,
                    })
                }
                _ => None,
            };
            game.apply(action)?;
            if let Some(flight) = flight {
                self.flights.push(flight);
            }
            self.next_launch = now + LAUNCH_INTERVAL;
        }
        Ok(self.actions.is_empty() && self.flights.is_empty())
    }

    pub fn moving_cards(&self) -> Vec<Card> {
        self.flights.iter().map(|flight| flight.card).collect()
    }

    pub fn paint(&self, ui: &egui::Ui, layout: BoardLayout, now: f64) {
        let painter = ui.painter();
        for flight in &self.flights {
            let t = ((now - flight.launched) / FLIGHT_DURATION).clamp(0.0, 1.0) as f32;
            let ease = t * t * (3.0 - 2.0 * t);
            let target = layout.foundation(flight.foundation);
            let size = target.size() * (1.0 + (std::f32::consts::PI * t).sin() * 0.07);
            let center = flight.from.center().lerp(target.center(), ease);
            let lift = (center.y - ui.clip_rect().min.y - size.y * 0.5 - 10.0).clamp(0.0, 85.0);
            let position = center - Vec2::new(0.0, (std::f32::consts::PI * t).sin() * lift);
            let rect = Rect::from_center_size(position, size);
            for halo in (1..=3).rev() {
                painter.rect_stroke(
                    rect.expand(halo as f32 * 3.0),
                    10.0,
                    Stroke::new(
                        2.0,
                        Color32::from_rgba_unmultiplied(239, 203, 119, (24 / halo) as u8),
                    ),
                    StrokeKind::Outside,
                );
            }
            theme::card(painter, rect, Some(flight.card), false);
        }
    }
}
