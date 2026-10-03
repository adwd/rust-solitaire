use crate::{
    input::{Drag, Interaction, foundation_move, source_selected},
    theme,
};
use eframe::egui::{
    self, Align2, FontId, Id, LayerId, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2,
};
use solitaire_core::{Action, Card, Game, Source, Suit, Target};

#[derive(Clone, Copy)]
struct Hit {
    rect: Rect,
    card_rect: Rect,
    source: Option<Source>,
    target: Option<Target>,
    stock: bool,
}

/// Render read-only game state; return a single command for the app to apply.
pub fn show(
    ui: &mut egui::Ui,
    game: &Game,
    interaction: &mut Interaction,
    hint: Option<Action>,
) -> Option<Action> {
    let view = game.view();
    let gap = 20.0;
    let width = ((ui.available_width() - 48.0 - gap * 6.0) / 7.0).clamp(64.0, 116.0);
    let height = width * 1.4;
    let step = width * 0.31;
    let hidden_step = width * 0.18;
    let tableau_y = height + 102.0;
    let pile_height = |pile: &[solitaire_core::TableauCard]| {
        pile.iter()
            .take(pile.len().saturating_sub(1))
            .map(|c| if c.face_up { step } else { hidden_step })
            .sum::<f32>()
            + height
    };
    let content_height = (tableau_y
        + view
            .tableau
            .iter()
            .map(|pile| pile_height(pile))
            .fold(height, f32::max)
        + 36.0)
        .max(ui.clip_rect().height());
    let (area, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), content_height),
        Sense::hover(),
    );
    let left = area.min.x + (area.width() - (width * 7.0 + gap * 6.0)) / 2.0;
    let pos =
        |column: usize, y: f32| Pos2::new(left + column as f32 * (width + gap), area.min.y + y);
    let rect = |column, y| Rect::from_min_size(pos(column, y), Vec2::new(width, height));
    let mut painter = ui.painter_at(area);
    // Keep stacked cards opaque under a modal; its backdrop supplies the dimming.
    painter.set_opacity(1.0);
    let mut hits = Vec::new();
    let mut targets = Vec::new();
    let selected = interaction.selected;
    let highlighted = |target: Target| {
        selected.is_some_and(|from| game.validate(Action::Move { from, to: target }).is_ok())
            || matches!(hint, Some(Action::Move { to, .. }) if to == target)
    };
    let label = |column, text: &str| {
        painter.text(
            pos(column, 12.0),
            Align2::LEFT_TOP,
            text,
            FontId::proportional(12.0),
            theme::MUTED,
        );
    };

    label(0, "STOCK");
    label(1, "WASTE");
    label(3, "FOUNDATIONS");
    let stock_rect = rect(0, 37.0);
    theme::slot(
        &painter,
        stock_rect,
        None,
        matches!(hint, Some(Action::Draw | Action::Recycle)),
    );
    if !view.stock.is_empty() {
        theme::card(&painter, stock_rect, None, false);
        painter.text(
            stock_rect.center(),
            Align2::CENTER_CENTER,
            view.stock.len().to_string(),
            FontId::proportional(22.0),
            theme::CREAM,
        );
    } else {
        painter.text(
            stock_rect.center(),
            Align2::CENTER_CENTER,
            "Recycle",
            FontId::proportional(15.0),
            theme::MUTED,
        );
    }
    if matches!(hint, Some(Action::Draw | Action::Recycle)) {
        painter.rect_stroke(
            stock_rect,
            8.0,
            Stroke::new(3.0, theme::GOLD),
            StrokeKind::Inside,
        );
    }
    hits.push(Hit {
        rect: stock_rect,
        card_rect: stock_rect,
        source: None,
        target: None,
        stock: true,
    });
    painter.text(
        stock_rect.center_bottom() + Vec2::new(0.0, 12.0),
        Align2::CENTER_TOP,
        format!("Draw {}", view.rules.draw.count()),
        FontId::proportional(12.0),
        theme::MUTED,
    );

    let waste_rect = rect(1, 37.0);
    theme::slot(&painter, waste_rect, None, false);
    let shown = view.waste.len().min(view.rules.draw.count());
    for (index, card) in view.waste.iter().skip(view.waste.len() - shown).enumerate() {
        let card_rect = waste_rect.translate(Vec2::new(index as f32 * width * 0.36, 0.0));
        let is_top = index + 1 == shown;
        if !(is_top
            && interaction
                .drag
                .is_some_and(|drag| drag.from == Source::Waste))
        {
            theme::card(
                &painter,
                card_rect,
                Some(*card),
                is_top && selected == Some(Source::Waste),
            );
        }
        if is_top {
            hits.push(Hit {
                rect: card_rect,
                card_rect,
                source: Some(Source::Waste),
                target: None,
                stock: false,
            });
        }
    }

    for (index, pile) in view.foundations.iter().enumerate() {
        let card_rect = rect(index + 3, 37.0);
        let target = Target::Foundation(index);
        theme::slot(
            &painter,
            card_rect,
            Some(Suit::ALL[index]),
            highlighted(target),
        );
        if let Some(card) = pile.last()
            && !interaction
                .drag
                .is_some_and(|drag| drag.from == Source::Foundation(index))
        {
            theme::card(
                &painter,
                card_rect,
                Some(*card),
                selected == Some(Source::Foundation(index)),
            );
        }
        if highlighted(target) {
            painter.rect_stroke(
                card_rect,
                8.0,
                Stroke::new(2.5, theme::LEGAL),
                StrokeKind::Inside,
            );
        }
        targets.push((target, card_rect));
        hits.push(Hit {
            rect: card_rect,
            card_rect,
            source: pile.last().map(|_| Source::Foundation(index)),
            target: Some(target),
            stock: false,
        });
        painter.text(
            card_rect.center_bottom() + Vec2::new(0.0, 12.0),
            Align2::CENTER_TOP,
            format!("{} / 13", pile.len()),
            FontId::proportional(12.0),
            theme::MUTED,
        );
    }

    for (column, pile) in view.tableau.iter().enumerate() {
        painter.text(
            pos(column, tableau_y - 23.0),
            Align2::LEFT_TOP,
            format!("0{}", column + 1),
            FontId::monospace(11.0),
            theme::MUTED,
        );
        let slot = rect(column, tableau_y);
        let target = Target::Tableau(column);
        let drop_rect = Rect::from_min_max(slot.min, Pos2::new(slot.max.x, area.max.y));
        targets.push((target, drop_rect));
        theme::slot(&painter, slot, None, highlighted(target));
        if pile.is_empty() {
            painter.text(
                slot.center(),
                Align2::CENTER_CENTER,
                "K",
                FontId::proportional(27.0),
                theme::MUTED,
            );
            hits.push(Hit {
                rect: slot,
                card_rect: slot,
                source: None,
                target: Some(target),
                stock: false,
            });
        }
        let mut y = tableau_y;
        for (index, placed) in pile.iter().enumerate() {
            let source = Source::Tableau { column, index };
            let card_rect = rect(column, y);
            let is_dragged = interaction
                .drag
                .is_some_and(|drag| source_selected(Some(drag.from), source));
            let is_hint = matches!(hint, Some(Action::Move { from, .. }) if source_selected(Some(from), source));
            if !is_dragged {
                theme::card(
                    &painter,
                    card_rect,
                    placed.face_up.then_some(placed.card),
                    source_selected(selected, source) || is_hint,
                );
            }
            let spacing = if placed.face_up { step } else { hidden_step };
            let hit_rect = if index + 1 == pile.len() {
                card_rect
            } else {
                Rect::from_min_size(card_rect.min, Vec2::new(width, spacing))
            };
            if placed.face_up {
                hits.push(Hit {
                    rect: hit_rect,
                    card_rect,
                    source: Some(source),
                    target: Some(target),
                    stock: false,
                });
            }
            y += spacing;
        }
        if highlighted(target) {
            let outline = Rect::from_min_size(slot.min, Vec2::new(width, pile_height(pile)));
            painter.rect_stroke(
                outline.expand(3.0),
                9.0,
                Stroke::new(2.0, theme::LEGAL),
                StrokeKind::Outside,
            );
        }
    }
    if matches!(
        hint,
        Some(Action::Move {
            from: Source::Waste,
            ..
        })
    ) && let Some(hit) = hits.iter().find(|hit| hit.source == Some(Source::Waste))
    {
        painter.rect_stroke(
            hit.card_rect,
            8.0,
            Stroke::new(3.0, theme::GOLD),
            StrokeKind::Inside,
        );
    }

    // A press and its first motion may arrive in the same native frame. Record
    // the card at the press origin rather than relying on the final hover hit.
    if ui.is_enabled() {
        let (pressed, down, released, origin, pointer) = ui.input(|i| {
            (
                i.pointer.button_pressed(egui::PointerButton::Primary),
                i.pointer.primary_down(),
                i.pointer.button_released(egui::PointerButton::Primary),
                i.pointer.press_origin(),
                i.pointer.interact_pos(),
            )
        });
        if pressed {
            interaction.pressed = origin.and_then(|origin| {
                hits.iter().find_map(|hit| {
                    (hit.rect.contains(origin) && ui.clip_rect().contains(origin))
                        .then_some(hit.source)
                        .flatten()
                        .map(|from| Drag {
                            from,
                            offset: hit.card_rect.min - origin,
                            origin,
                        })
                })
            });
        }
        if (down || released)
            && let Some(drag) = interaction.pressed
            && pointer.is_some_and(|pointer| drag.origin.distance(pointer) > 6.0)
        {
            interaction.pressed = None;
            interaction.selected = Some(drag.from);
            interaction.drag = Some(drag);
        }
        if released {
            interaction.pressed = None;
        }
    }

    let mut action = None;
    for (index, hit) in hits.iter().enumerate() {
        let response = ui.interact(
            hit.rect,
            ui.id().with(("card", index)),
            if hit.source.is_some() {
                Sense::click_and_drag()
            } else {
                Sense::click()
            },
        );
        let accessible_label = match hit.source {
            Some(Source::Tableau { column, index }) => format!(
                "{} in column {}",
                view.tableau[column][index].card,
                column + 1
            ),
            Some(Source::Waste) => format!("Waste: {}", view.waste.last().expect("visible waste")),
            Some(Source::Foundation(index)) => format!(
                "Foundation {}: {}",
                index + 1,
                view.foundations[index].last().expect("visible foundation")
            ),
            None if hit.stock => {
                if view.stock.is_empty() {
                    "Recycle stock".into()
                } else {
                    "Draw from stock".into()
                }
            }
            None => match hit.target {
                Some(Target::Tableau(index)) => format!("Empty column {}", index + 1),
                Some(Target::Foundation(index)) => {
                    format!("Empty {:?} foundation", Suit::ALL[index])
                }
                None => "Empty pile".into(),
            },
        };
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                ui.is_enabled(),
                accessible_label.as_str(),
            )
        });
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if response.drag_started_by(egui::PointerButton::Primary)
            && interaction.drag.is_none()
            && let Some(from) = hit.source
        {
            let origin = ui
                .input(|i| i.pointer.press_origin())
                .unwrap_or(hit.card_rect.min);
            interaction.start_drag(from, origin, hit.card_rect.min);
        }
        if response.double_clicked()
            && let Some(from) = hit.source
        {
            action = foundation_move(game, from);
            interaction.clear();
        } else if response.clicked() {
            if hit.stock {
                let candidate = if view.stock.is_empty() {
                    Action::Recycle
                } else {
                    Action::Draw
                };
                if game.validate(candidate).is_ok() {
                    action = Some(candidate);
                }
                interaction.clear();
            } else {
                let candidate = interaction
                    .selected
                    .zip(hit.target)
                    .map(|(from, to)| Action::Move { from, to });
                if let Some(candidate) = candidate.filter(|action| game.validate(*action).is_ok()) {
                    action = Some(candidate);
                    interaction.clear();
                } else {
                    interaction.select(hit.source);
                }
            }
        }
    }

    if ui.is_enabled()
        && let Some(drag) = interaction.drag
    {
        let pointer = ui.input(|i| i.pointer.interact_pos());
        if let Some(pointer) = pointer {
            let cards: Vec<Card> = match drag.from {
                Source::Waste => view.waste.last().copied().into_iter().collect(),
                Source::Foundation(index) => view.foundations[index]
                    .last()
                    .copied()
                    .into_iter()
                    .collect(),
                Source::Tableau { column, index } => view.tableau[column][index..]
                    .iter()
                    .map(|placed| placed.card)
                    .collect(),
            };
            let overlay = ui
                .ctx()
                .layer_painter(LayerId::new(Order::Foreground, Id::new("dragged-cards")));
            for (index, card) in cards.iter().enumerate() {
                let card_rect = Rect::from_min_size(
                    pointer + drag.offset + Vec2::new(0.0, index as f32 * step),
                    Vec2::new(width, height),
                );
                theme::card(&overlay, card_rect, Some(*card), true);
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
                action = targets.iter().find_map(|(to, rect)| {
                    let candidate = Action::Move {
                        from: drag.from,
                        to: *to,
                    };
                    (rect.contains(pointer) && game.validate(candidate).is_ok())
                        .then_some(candidate)
                });
                interaction.clear();
            }
        } else {
            interaction.clear();
        }
    }
    action
}

#[cfg(test)]
mod tests {
    use super::*;
    use solitaire_core::Rules;

    struct Harness {
        ctx: egui::Context,
        game: Game,
        interaction: Interaction,
        time: f64,
    }

    impl Harness {
        fn new(game: Game) -> Self {
            let mut result = Self {
                ctx: egui::Context::default(),
                game,
                interaction: Interaction::default(),
                time: 0.0,
            };
            result.frame(vec![]);
            result.frame(vec![]);
            result
        }

        fn frame(&mut self, events: Vec<egui::Event>) {
            self.time += 0.1;
            let mut action = None;
            let _output = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1100.0, 800.0))),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ui| {
                    action = show(ui, &self.game, &mut self.interaction, None);
                },
            );
            if let Some(action) = action {
                self.game.apply(action).unwrap();
            }
        }

        fn button(pos: Pos2, pressed: bool) -> egui::Event {
            egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Primary,
                modifiers: egui::Modifiers::NONE,
            }
        }

        fn click(&mut self, pos: Pos2) {
            self.frame(vec![
                egui::Event::PointerMoved(pos),
                Self::button(pos, true),
            ]);
            self.frame(vec![Self::button(pos, false)]);
            self.frame(vec![]);
        }

        fn point(&self, source: Source) -> Pos2 {
            // Integration fixture uses the default 1100-point window with 116-point cards.
            let Source::Tableau { column, index } = source else {
                panic!("tableau fixture")
            };
            let y: f32 = self.game.view().tableau[column][..index]
                .iter()
                .map(|card| if card.face_up { 35.96 } else { 20.88 })
                .sum();
            Pos2::new(84.0 + column as f32 * 136.0 + 58.0, 264.4 + y + 12.0)
        }
    }

    fn movable_deal() -> (Game, Source, usize) {
        for seed in 0..200 {
            let game = Game::new(seed, Rules::default());
            if let Some(Action::Move {
                from,
                to: Target::Tableau(column),
            }) = game.legal_actions().into_iter().find(|action| {
                matches!(
                    action,
                    Action::Move {
                        from: Source::Tableau { .. },
                        to: Target::Tableau(_)
                    }
                )
            }) {
                return (game, from, column);
            }
        }
        panic!("a movable initial deal exists");
    }

    #[test]
    fn stock_click_is_one_core_action() {
        let mut h = Harness::new(Game::new(42, Rules::default()));
        h.click(Pos2::new(142.0, 90.0));
        assert_eq!(h.game.view().stock.len(), 23);
        assert_eq!(h.game.view().waste.len(), 1);
        assert_eq!(h.game.view().moves, 1);
    }

    #[test]
    fn clicking_source_then_destination_moves_card() {
        let (game, from, to) = movable_deal();
        let mut h = Harness::new(game);
        let before = h.game.view().tableau[to].len();
        let from_point = h.point(from);
        let top = h.game.view().tableau[to].len().saturating_sub(1);
        let to_point = h.point(Source::Tableau {
            column: to,
            index: top,
        });
        h.click(from_point);
        assert_eq!(h.interaction.selected, Some(from));
        h.click(to_point);
        assert_eq!(h.game.view().moves, 1);
        assert_eq!(h.game.view().tableau[to].len(), before + 1);
        assert!(h.interaction.selected.is_none());
    }

    #[test]
    fn drag_commits_once_and_invalid_drop_cancels() {
        let (game, from, to) = movable_deal();
        let mut h = Harness::new(game);
        let origin = h.point(from);
        h.frame(vec![
            egui::Event::PointerMoved(origin),
            Harness::button(origin, true),
        ]);
        h.frame(vec![egui::Event::PointerMoved(
            origin + Vec2::new(10.0, 10.0),
        )]);
        assert!(h.interaction.drag.is_some());
        assert_eq!(h.game.view().moves, 0);
        let target = Pos2::new(84.0 + to as f32 * 136.0 + 58.0, 650.0);
        h.frame(vec![egui::Event::PointerMoved(target)]);
        h.frame(vec![Harness::button(target, false)]);
        assert_eq!(h.game.view().moves, 1);
        assert!(h.interaction.drag.is_none());
        h.game.undo();
        h.frame(vec![]);
        h.frame(vec![
            egui::Event::PointerMoved(origin),
            Harness::button(origin, true),
        ]);
        h.frame(vec![egui::Event::PointerMoved(
            origin + Vec2::new(10.0, 10.0),
        )]);
        let outside = Pos2::new(5.0, 5.0);
        h.frame(vec![egui::Event::PointerMoved(outside)]);
        h.frame(vec![Harness::button(outside, false)]);
        assert_eq!(h.game.view().moves, 0);
        assert!(h.game.can_redo());
        assert!(h.interaction.drag.is_none());
    }

    #[test]
    fn drag_with_press_and_motion_in_one_frame_moves_card() {
        let (game, from, to) = movable_deal();
        let mut h = Harness::new(game);
        let origin = h.point(from);
        let target = Pos2::new(84.0 + to as f32 * 136.0 + 58.0, 550.0);
        h.frame(vec![
            egui::Event::PointerMoved(origin),
            Harness::button(origin, true),
            egui::Event::PointerMoved(target),
        ]);
        h.frame(vec![Harness::button(target, false)]);
        assert_eq!(h.game.view().moves, 1);
        assert!(h.interaction.drag.is_none());
    }

    #[test]
    fn drag_with_first_motion_and_release_in_one_frame_moves_card() {
        let (game, from, to) = movable_deal();
        let mut h = Harness::new(game);
        let origin = h.point(from);
        let target = Pos2::new(84.0 + to as f32 * 136.0 + 58.0, 550.0);
        h.frame(vec![
            egui::Event::PointerMoved(origin),
            Harness::button(origin, true),
        ]);
        h.frame(vec![
            egui::Event::PointerMoved(target),
            Harness::button(target, false),
        ]);
        assert_eq!(h.game.view().moves, 1);
        assert!(h.interaction.drag.is_none());
        assert!(h.interaction.pressed.is_none());
    }

    #[test]
    fn dragging_an_exposed_strip_moves_the_whole_sequence() {
        let mut game = Game::new(6, Rules::default());
        let preparatory = [
            Action::Move {
                from: Source::Tableau {
                    column: 1,
                    index: 1,
                },
                to: Target::Foundation(2),
            },
            Action::Move {
                from: Source::Tableau {
                    column: 4,
                    index: 4,
                },
                to: Target::Tableau(5),
            },
            Action::Move {
                from: Source::Tableau {
                    column: 2,
                    index: 2,
                },
                to: Target::Tableau(4),
            },
            Action::Move {
                from: Source::Tableau {
                    column: 2,
                    index: 1,
                },
                to: Target::Tableau(1),
            },
        ];
        for action in preparatory {
            game.apply(action).unwrap();
        }
        let from = Source::Tableau {
            column: 4,
            index: 3,
        };
        let count = game.view().tableau[4].len() - 3;
        assert!(count > 1);
        let before = game.view().tableau[0].len();
        let mut h = Harness::new(game);
        let origin = h.point(from);
        h.frame(vec![
            egui::Event::PointerMoved(origin),
            Harness::button(origin, true),
        ]);
        h.frame(vec![egui::Event::PointerMoved(
            origin + Vec2::new(10.0, 10.0),
        )]);
        let target = Pos2::new(142.0, 650.0);
        h.frame(vec![egui::Event::PointerMoved(target)]);
        h.frame(vec![Harness::button(target, false)]);
        assert_eq!(h.game.view().tableau[0].len(), before + count);
        assert_eq!(h.game.view().moves, 5);
        assert!(h.game.view().tableau[4].last().unwrap().face_up);
    }
}
