use crate::{board, input::Interaction, theme};
use eframe::egui::{self, Color32, Id, Key, Modifiers, RichText};
use solitaire_core::{Action, DrawMode, Game, Rules, Source, Status, Target};
use std::time::{Duration, Instant};

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
}

impl SolitaireApp {
    pub fn new(cc: &eframe::CreationContext<'_>, seed: u64, rules: Rules) -> Self {
        theme::setup(&cc.egui_ctx);
        if let Some(render) = &cc.wgpu_render_state {
            let adapter = render.adapter.get_info();
            eprintln!(
                "rust-solitaire renderer: wgpu / {:?} / {}",
                adapter.backend, adapter.name
            );
        }
        Self {
            game: Game::new(seed, rules),
            interaction: Interaction::default(),
            clock: Clock::default(),
            hint: None,
            hint_counter: 0,
            message: "Drag cards, or select a card and click its destination.".into(),
            dialog: None,
            victory_dismissed: false,
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
        self.message = "New game started.".into();
    }

    fn apply(&mut self, action: Action) {
        match self.game.apply(action) {
            Ok(status) => {
                self.clock.start();
                if status == Status::Won {
                    self.clock.stop();
                }
                self.hint = None;
                self.hint_counter = 0;
                self.interaction.clear();
                self.message = match action {
                    Action::Draw => "Drew from the stock.".into(),
                    Action::Recycle => "Recycled the waste into the stock.".into(),
                    Action::Move { .. } => "Moved cards.".into(),
                };
            }
            Err(error) => {
                self.message = format!("Cannot move: {error}");
            }
        }
    }

    fn undo(&mut self) {
        if self.game.undo() {
            self.interaction.clear();
            self.hint = None;
            self.hint_counter = 0;
            self.clock.start();
            self.victory_dismissed = false;
            self.message = "Undid one move.".into();
        }
    }

    fn redo(&mut self) {
        if self.game.redo() {
            self.interaction.clear();
            self.hint = None;
            self.hint_counter = 0;
            self.clock.start();
            if self.game.status() == Status::Won {
                self.clock.stop();
            }
            self.message = "Redid one move.".into();
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
            self.redo();
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
                                    start = Some((seed.unwrap_or_else(rand::random), *draw))
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
        if self.game.status() != Status::Won || self.victory_dismissed || self.dialog.is_some() {
            return;
        }
        let mut next = false;
        let mut back = false;
        egui::Modal::new(Id::new("victory")).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(RichText::new("COMPLETE").size(13.0).color(theme::GOLD));
            ui.heading("You won!");
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
                        if ui.button("New game").clicked() {
                            self.new_dialog();
                        }
                        if ui.button("Restart").clicked() {
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
                            .add_enabled(self.game.can_redo(), egui::Button::new("Redo"))
                            .on_hover_text("Cmd / Ctrl + Shift + Z")
                            .clicked()
                        {
                            self.redo();
                        }
                        if ui
                            .add_enabled(
                                self.game.status() != Status::Won,
                                egui::Button::new("Hint"),
                            )
                            .clicked()
                        {
                            self.show_hint();
                        }
                        if ui.button("How to play").clicked() {
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
                    "Drop onto a highlighted pile. Press Esc to cancel."
                } else if self.interaction.selected.is_some() {
                    "Click a highlighted destination. Press Esc to clear your selection."
                } else {
                    &self.message
                };
                ui.label(RichText::new(text).size(13.0).color(theme::MUTED));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::FELT).inner_margin(12))
            .show(ui, |ui| {
                ui.add_enabled_ui(
                    self.dialog.is_none() && self.game.status() != Status::Won,
                    |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if let Some(action) =
                                    board::show(ui, &self.game, &mut self.interaction, self.hint)
                                {
                                    self.apply(action);
                                }
                            });
                    },
                );
            });
        self.dialogs(&ctx);
        self.victory(&ctx);
        if self.clock.running.is_some() {
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
