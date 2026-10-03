use crate::{board, celebration, finish::FinishAnimation, input::Interaction, theme};
use eframe::egui::{self, Color32, Id, Key, Modifiers, RichText};
use solitaire_core::{Action, DrawMode, Game, Rules, Source, Status, Target};
use web_time::{Duration, Instant};

#[derive(Default)]
struct Clock {
    elapsed: Duration,
    running: Option<Instant>,
}

impl Clock {
    fn elapsed(&self) -> Duration {
        self.elapsed + self.running.map_or(Duration::ZERO, |start| start.elapsed())
    }
    fn start(&mut self) {
        if self.running.is_none() {
            self.running = Some(Instant::now());
        }
    }
    fn stop(&mut self) {
        if let Some(start) = self.running.take() {
            self.elapsed += start.elapsed();
        }
    }
}

enum Dialog {
    New {
        draw: DrawMode,
        seed: String,
        error: Option<String>,
    },
    Restart,
    Help,
}

pub struct SolitaireApp {
    game: Game,
    interaction: Interaction,
    clock: Clock,
    hint: Option<Action>,
    hint_counter: usize,
    message: String,
    dialog: Option<Dialog>,
    victory_dismissed: bool,
    finish: Option<FinishAnimation>,
    celebration_started: Option<f64>,
}

impl SolitaireApp {
    pub fn new(cc: &eframe::CreationContext<'_>, seed: u64, rules: Rules) -> Self {
        Self::with_game(cc, Game::new(seed, rules))
    }

    /// Embed a prepared core game with the same screen and controls.
    pub fn with_game(cc: &eframe::CreationContext<'_>, game: Game) -> Self {
        theme::setup(&cc.egui_ctx);
        if let Some(render) = &cc.wgpu_render_state {
            celebration::register(render);
            let adapter = render.adapter.get_info();
            eprintln!(
                "rust-solitaire renderer: wgpu / {:?} / {}",
                adapter.backend, adapter.name
            );
        }
        Self {
            game,
            interaction: Interaction::default(),
            clock: Clock::default(),
            hint: None,
            hint_counter: 0,
            message: "Drag cards, or select a card and click its destination.".into(),
            dialog: None,
            victory_dismissed: false,
            finish: None,
            celebration_started: None,
        }
    }

    fn begin(&mut self, seed: u64, draw: DrawMode) {
        self.game = Game::new(seed, Rules { draw });
        self.clock = Clock::default();
        self.interaction.clear();
        self.hint = None;
        self.hint_counter = 0;
        self.dialog = None;
        self.victory_dismissed = false;
        self.finish = None;
        self.celebration_started = None;
        self.message = "New game started.".into();
    }

    fn apply(&mut self, action: Action, now: f64) {
        match self.game.apply(action) {
            Ok(_) => {
                self.clock.start();
                self.hint = None;
                self.hint_counter = 0;
                self.interaction.clear();
                self.message = match action {
                    Action::Draw => "Drew from the stock.".into(),
                    Action::Recycle => "Recycled the waste into the stock.".into(),
                    Action::Move { .. } => "Moved cards.".into(),
                };
                self.check_finish(now);
            }
            Err(error) => {
                self.message = format!("Cannot move: {error}");
            }
        }
    }

    fn undo(&mut self) {
        if self.game.undo() {
            self.finish = None;
            self.celebration_started = None;
            self.interaction.clear();
            self.hint = None;
            self.hint_counter = 0;
            self.clock.start();
            self.victory_dismissed = false;
            self.message = "Undid one move.".into();
        }
    }

    fn redo(&mut self, now: f64) {
        if self.game.redo() {
            self.interaction.clear();
            self.hint = None;
            self.hint_counter = 0;
            self.clock.start();
            self.message = "Redid one move.".into();
            self.check_finish(now);
        }
    }

    fn check_finish(&mut self, now: f64) {
        if self.game.status() == Status::Won {
            self.clock.stop();
            self.celebration_started = Some(now);
            self.message = "Deal complete. Nicely played!".into();
        } else if let Some(plan) = self.game.auto_finish_plan() {
            self.clock.stop();
            self.interaction.clear();
            self.hint = None;
            self.finish = Some(FinishAnimation::new(plan, now));
            self.message = "All set! Finishing your deal… Press Undo to take control.".into();
        }
    }

    fn advance_finish(&mut self, layout: board::BoardLayout, now: f64) {
        let Some(mut finish) = self.finish.take() else {
            return;
        };
        match finish.advance(&mut self.game, layout, now) {
            Ok(true) if self.game.status() == Status::Won => {
                self.celebration_started = Some(now);
                self.message = "Deal complete. Nicely played!".into();
            }
            Ok(false) => self.finish = Some(finish),
            _ => {
                self.clock.start();
                self.message = "Automatic finish stopped. You can keep playing.".into();
            }
        }
    }

    fn new_dialog(&mut self) {
        self.interaction.clear();
        self.hint = None;
        self.dialog = Some(Dialog::New {
            draw: self.game.view().rules.draw,
            seed: String::new(),
            error: None,
        });
    }

    fn restart(&mut self) {
        if self.game.view().moves == 0 {
            self.begin(self.game.view().seed, self.game.view().rules.draw);
        } else {
            self.interaction.clear();
            self.dialog = Some(Dialog::Restart);
        }
    }

    fn show_hint(&mut self) {
        self.interaction.clear();
        let view = self.game.view();
        let mut choices = self.game.legal_actions();
        choices.sort_by_key(|action| match action {
            Action::Move {
                to: Target::Foundation(_),
                ..
            } => 0,
            Action::Move {
                from: Source::Tableau { column, index },
                ..
            } if *index > 0 && !view.tableau[*column][index - 1].face_up => 1,
            Action::Move {
                from: Source::Waste,
                ..
            } => 2,
            Action::Move {
                from: Source::Tableau { .. },
                ..
            } => 3,
            Action::Draw | Action::Recycle => 4,
            Action::Move {
                from: Source::Foundation(_),
                ..
            } => 5,
        });
        if choices.is_empty() {
            self.hint = None;
            self.message = "No legal actions remain. Undo a move or try a new deal.".into();
        } else {
            let action = choices[self.hint_counter % choices.len()];
            self.hint_counter += 1;
            self.hint = Some(action);
            self.message = match action {
                Action::Draw => "Hint: draw from the stock.".into(),
                Action::Recycle => "Hint: recycle the waste into the stock.".into(),
                Action::Move { from, to } => {
                    let from = match from {
                        Source::Waste => "Waste".into(),
                        Source::Foundation(index) => format!("Foundation {}", index + 1),
                        Source::Tableau { column, .. } => format!("Column {}", column + 1),
                    };
                    let to = match to {
                        Target::Tableau(i) => format!("Column {}", i + 1),
                        Target::Foundation(i) => format!("Foundation {}", i + 1),
                    };
                    format!("Hint: {from} to {to}. Click Hint again for another legal action.")
                }
            };
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| !i.focused) {
            self.interaction.clear();
        }
        if self.dialog.is_some() {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)) {
            if self.finish.is_none() {
                self.redo(ctx.input(|i| i.time));
            }
        } else if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
            self.undo();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            self.interaction.clear();
            self.hint = None;
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let mut close = false;
        let mut start = None;
        let response = egui::Modal::new(Id::new("game-dialog")).show(ctx, |ui| {
            ui.set_width(390.0);
            match &mut dialog {
                Dialog::New { draw, seed, error } => {
                    ui.heading("New game");
                    ui.add_space(8.0);
                    ui.label("Draw mode");
                    ui.horizontal(|ui| {
                        ui.selectable_value(draw, DrawMode::One, "Draw 1");
                        ui.selectable_value(draw, DrawMode::Three, "Draw 3");
                    });
                    ui.add_space(6.0);
                    ui.label("Deal seed (leave empty for a random deal)");
                    ui.add(
                        egui::TextEdit::singleline(seed)
                            .hint_text("e.g. 42")
                            .desired_width(f32::INFINITY),
                    );
                    if let Some(error) = error {
                        ui.colored_label(Color32::LIGHT_RED, error.as_str());
                    }
                    if self.game.view().moves > 0 {
                        ui.label(
                            RichText::new("Starting a new game resets your current game and history.")
                                .small()
                                .color(theme::MUTED),
                        );
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Start game").clicked() {
                            match parse_seed(seed) {
                                Ok(seed) => {
                                    start = Some((seed.unwrap_or_else(crate::random_seed), *draw))
                                }
                                Err(reason) => *error = Some(reason.into()),
                            }
                        }
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Restart => {
                    ui.heading("Restart this deal?");
                    ui.add_space(8.0);
                    ui.label("Reset your current game and history, keeping the same seed and draw mode.");
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Restart").clicked() {
                            start = Some((self.game.view().seed, self.game.view().rules.draw));
                        }
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Help => {
                    ui.heading("How to play");
                    ui.add_space(8.0);
                    for text in [
                        "Build tableau columns in descending order, alternating red and black.",
                        "Move a valid sequence of face-up cards together.",
                        "Only a King or a sequence starting with a King fits an empty column.",
                        "Build each foundation from Ace to King in one suit. Collect all 52 cards to win.",
                        "Only the top waste card is playable. Recycle the stock as often as you like.",
                        "Drag cards, or select a card and click its destination.",
                        "Double-click to move to a foundation. Esc cancels your selection.",
                        "Cmd / Ctrl + Z: Undo. Add Shift to Redo.",
                        "Choose Draw 1 / Draw 3 and a seed in New game.",
                    ] {
                        ui.label(text);
                    }
                    ui.add_space(12.0);
                    close = ui.button("Close").clicked();
                }
            }
        });
        if let Some((seed, draw)) = start {
            self.begin(seed, draw);
        } else if !close && !response.should_close() {
            self.dialog = Some(dialog);
        }
    }

    fn victory(&mut self, ctx: &egui::Context) {
        if self.game.status() != Status::Won
            || self.finish.is_some()
            || self.victory_dismissed
            || self.dialog.is_some()
        {
            return;
        }
        let mut next = false;
        let mut back = false;
        egui::Modal::new(Id::new("victory"))
            .backdrop_color(Color32::from_black_alpha(45))
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .corner_radius(16)
                    .stroke(egui::Stroke::new(1.5, theme::GOLD))
                    .inner_margin(24),
            )
            .show(ctx, |ui| {
                ui.set_width(400.0);
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("D E A L   C O M P L E T E")
                            .size(13.0)
                            .color(theme::GOLD),
                    );
                    ui.label(
                        RichText::new("You won!")
                            .size(46.0)
                            .strong()
                            .color(theme::CREAM),
                    );
                    ui.label(RichText::new("52 cards. One great finish.").color(theme::MUTED));
                });
                ui.add_space(8.0);
                ui.label(format!(
                    "{} moves  |  {}  |  Seed {}",
                    self.game.view().moves,
                    format_time(self.clock.elapsed()),
                    self.game.view().seed
                ));
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    next = ui.button("New game").clicked();
                    back = ui.button("View board").clicked();
                });
            });
        if next {
            self.new_dialog();
        }
        if back {
            self.victory_dismissed = true;
        }
    }
}

impl eframe::App for SolitaireApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        self.shortcuts(&ctx);
        egui::Panel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .inner_margin(egui::Margin::symmetric(24, 16)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("RUST").size(12.0).color(theme::GOLD).strong());
                    ui.label(RichText::new("SOLITAIRE").size(25.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let v = self.game.view();
                        let completed: usize = v.foundations.iter().map(Vec::len).sum();
                        ui.label(
                            RichText::new(format!("{completed} / 52"))
                                .size(17.0)
                                .color(theme::GOLD),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new(format_time(self.clock.elapsed()))
                                .monospace()
                                .size(17.0),
                        );
                        ui.separator();
                        ui.label(format!("{} moves", v.moves));
                    });
                });
                ui.add_space(10.0);
                ui.add_enabled_ui(self.dialog.is_none(), |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .add_enabled(self.finish.is_none(), egui::Button::new("New game"))
                            .clicked()
                        {
                            self.new_dialog();
                        }
                        if ui
                            .add_enabled(self.finish.is_none(), egui::Button::new("Restart"))
                            .clicked()
                        {
                            self.restart();
                        }
                        ui.separator();
                        if ui
                            .add_enabled(self.game.can_undo(), egui::Button::new("Undo"))
                            .on_hover_text("Cmd / Ctrl + Z")
                            .clicked()
                        {
                            self.undo();
                        }
                        if ui
                            .add_enabled(
                                self.finish.is_none() && self.game.can_redo(),
                                egui::Button::new("Redo"),
                            )
                            .on_hover_text("Cmd / Ctrl + Shift + Z")
                            .clicked()
                        {
                            self.redo(now);
                        }
                        if ui
                            .add_enabled(
                                self.finish.is_none() && self.game.status() != Status::Won,
                                egui::Button::new("Hint"),
                            )
                            .clicked()
                        {
                            self.show_hint();
                        }
                        if ui
                            .add_enabled(self.finish.is_none(), egui::Button::new("How to play"))
                            .clicked()
                        {
                            self.interaction.clear();
                            self.dialog = Some(Dialog::Help);
                        }
                        let v = self.game.view();
                        ui.label(
                            RichText::new(format!(
                                "Draw {}  |  SEED {}",
                                v.rules.draw.count(),
                                v.seed
                            ))
                            .small()
                            .color(theme::MUTED),
                        );
                    });
                });
            });
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .inner_margin(egui::Margin::symmetric(24, 12)),
            )
            .show(ui, |ui| {
                let text = if self.interaction.drag.is_some() {
                    "Drop onto a pile. Press Esc to cancel."
                } else if self.interaction.selected.is_some() {
                    "Click a destination. Press Esc to clear your selection."
                } else {
                    &self.message
                };
                ui.label(RichText::new(text).size(13.0).color(theme::MUTED));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::FELT).inner_margin(12))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let layout = board::layout(ui, &self.game);
                        self.advance_finish(layout, now);
                        let moving = self
                            .finish
                            .as_ref()
                            .map_or_else(Vec::new, FinishAnimation::moving_cards);
                        let action = ui
                            .add_enabled_ui(
                                self.dialog.is_none()
                                    && self.finish.is_none()
                                    && self.game.status() != Status::Won,
                                |ui| {
                                    board::show(
                                        ui,
                                        &self.game,
                                        &mut self.interaction,
                                        self.hint,
                                        layout,
                                        &moving,
                                    )
                                },
                            )
                            .inner;
                        if let Some(action) = action {
                            self.apply(action, now);
                        }
                        if let Some(finish) = &self.finish {
                            finish.paint(ui, layout, now);
                        }
                    });
                if let Some(started) = self.celebration_started {
                    celebration::paint(ui, now - started);
                }
            });
        self.dialogs(&ctx);
        self.victory(&ctx);
        if self.finish.is_some() {
            ctx.request_repaint();
        } else if self.clock.running.is_some() {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

fn parse_seed(value: &str) -> Result<Option<u64>, &'static str> {
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        value
            .parse()
            .map(Some)
            .map_err(|_| "Enter an integer between 0 and 18446744073709551615.")
    }
}

fn format_time(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(moves: usize) -> Game {
        let mut game = Game::new(6, Rules::default());
        for line in include_str!("../../solitaire-core/tests/fixtures/seed-6.moves")
            .lines()
            .take(moves)
        {
            let action = match line {
                "D" => Action::Draw,
                "R" => Action::Recycle,
                _ => {
                    let (from, to) = line.split_once('>').unwrap();
                    let source = if from == "W" {
                        Source::Waste
                    } else {
                        let (column, index) = from[1..].split_once(':').unwrap();
                        Source::Tableau {
                            column: column.parse().unwrap(),
                            index: index.parse().unwrap(),
                        }
                    };
                    let target = to[1..].parse().unwrap();
                    Action::Move {
                        from: source,
                        to: if to.starts_with('F') {
                            Target::Foundation(target)
                        } else {
                            Target::Tableau(target)
                        },
                    }
                }
            };
            game.apply(action).unwrap();
        }
        game
    }

    fn app(game: Game) -> SolitaireApp {
        SolitaireApp {
            game,
            interaction: Interaction::default(),
            clock: Clock {
                elapsed: Duration::from_secs(91),
                running: Some(Instant::now()),
            },
            hint: None,
            hint_counter: 0,
            message: String::new(),
            dialog: None,
            victory_dismissed: false,
            finish: None,
            celebration_started: None,
        }
    }

    fn layout(game: &Game) -> board::BoardLayout {
        let ctx = egui::Context::default();
        let mut layout = None;
        let _output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1100.0, 800.0),
                )),
                ..Default::default()
            },
            |ui| layout = Some(board::layout(ui, game)),
        );
        layout.unwrap()
    }

    #[test]
    fn clock_freezes_at_proof_and_victory_waits_for_the_last_flight() {
        let mut app = app(fixture(208));
        app.check_finish(10.0);
        let frozen = app.clock.elapsed();
        assert!(app.clock.running.is_none());
        assert!(app.finish.is_some());
        assert!(app.celebration_started.is_none());
        let layout = layout(&app.game);
        let mut saw_last_flight = false;
        for frame in 0..100 {
            app.advance_finish(layout, 10.0 + f64::from(frame) * 0.06);
            assert_eq!(app.clock.elapsed(), frozen);
            if app.game.status() == Status::Won && app.finish.is_some() {
                saw_last_flight = true;
                assert!(app.celebration_started.is_none());
            }
        }
        assert!(saw_last_flight);
        assert_eq!(app.game.status(), Status::Won);
        assert!(app.finish.is_none());
        assert!(app.celebration_started.is_some());
        assert_eq!(app.game.view().moves, 234);
    }

    #[test]
    fn undo_cancels_automatic_play_and_resumes_the_clock() {
        let mut app = app(fixture(208));
        app.check_finish(10.0);
        let layout = layout(&app.game);
        app.advance_finish(layout, 10.25);
        assert_eq!(app.game.view().moves, 209);
        app.undo();
        assert_eq!(app.game.view().moves, 208);
        assert!(app.finish.is_none());
        assert!(app.celebration_started.is_none());
        assert!(app.clock.running.is_some());
        app.advance_finish(layout, 11.0);
        assert_eq!(app.game.view().moves, 208);
        app.redo(11.1);
        assert!(app.finish.is_some());
        assert!(app.clock.running.is_none());
    }

    #[test]
    fn suspended_animation_does_not_skip_the_card_cascade() {
        let mut app = app(fixture(208));
        app.check_finish(1.0);
        let layout = layout(&app.game);
        app.advance_finish(layout, 100.0);
        assert_eq!(app.game.view().moves, 209);
        assert!(app.finish.is_some());
        app.advance_finish(layout, 100.001);
        assert_eq!(app.game.view().moves, 209);
    }

    #[test]
    fn new_deal_resets_finish_effects_and_frozen_time() {
        let mut app = app(fixture(208));
        app.check_finish(10.0);
        app.celebration_started = Some(10.0);
        app.begin(42, DrawMode::Three);
        assert!(app.finish.is_none());
        assert!(app.celebration_started.is_none());
        assert_eq!(app.clock.elapsed(), Duration::ZERO);
        assert_eq!(app.game.view().moves, 0);
    }
}
