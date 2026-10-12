use crate::theme;
use eframe::egui::{self, Color32, Id, Key, Modifiers, Rect, RichText, Sense, Stroke, Vec2};
use sokoban_core::{Direction, Game, LEVEL_COUNT, Position, Size, Source, Tile, level_name};

enum Dialog {
    Help,
    Start(Source),
    Restart,
    Random {
        seed: String,
        size: Size,
        error: Option<String>,
    },
}

pub struct SokobanApp {
    game: Game,
    dialog: Option<Dialog>,
    message: String,
    board_rect: Option<Rect>,
}

impl SokobanApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::setup(&cc.egui_ctx);
        Self::with_game(Game::default())
    }

    pub fn with_game(game: Game) -> Self {
        Self {
            game,
            dialog: None,
            board_rect: None,
            message: "Push every crate onto a gold goal. Crates can only be pushed, never pulled."
                .into(),
        }
    }

    fn start(&mut self, source: Source) {
        let game = match source {
            Source::Level(level) => Game::new(level),
            Source::Random { seed, size } => Some(Game::random(seed, size)),
        };
        if let Some(game) = game {
            self.game = game;
            self.dialog = None;
            self.message =
                "Use arrows, WASD, the direction buttons, or click beside the player.".into();
        }
    }

    fn request_start(&mut self, source: Source) {
        if self.game.moves() > 0 && !self.game.is_won() {
            self.dialog = Some(Dialog::Start(source));
        } else {
            self.start(source);
        }
    }

    fn step(&mut self, direction: Direction) {
        if self.game.apply(direction) {
            self.message = if self.game.is_won() {
                "All crates delivered! Choose the next level, or undo to try a different finish."
            } else {
                "A crate stuck against a wall? Use Undo or Restart to try again."
            }
            .into();
        } else if !self.game.is_won() {
            self.message = "Blocked. You can push one crate into an empty square.".into();
        }
    }

    fn undo(&mut self) {
        if self.game.undo() {
            self.message = "Undid one move.".into();
        }
    }

    fn redo(&mut self) {
        if self.game.redo() {
            self.message = "Redid one move.".into();
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() || egui::Popup::is_any_open(ctx) {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)) {
            self.redo();
        } else if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
            self.undo();
        }
        for (keys, direction) in [
            ([Key::ArrowUp, Key::W], Direction::Up),
            ([Key::ArrowDown, Key::S], Direction::Down),
            ([Key::ArrowLeft, Key::A], Direction::Left),
            ([Key::ArrowRight, Key::D], Direction::Right),
        ] {
            if keys
                .into_iter()
                .any(|key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)))
            {
                self.step(direction);
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("RUST").size(12.0).color(theme::GOLD).strong());
            ui.label(RichText::new("Sokoban").size(25.0).strong());
            ui.separator();
            ui.label(format!(
                "{} moves  /  {} pushes",
                self.game.moves(),
                self.game.pushes()
            ));
            ui.label(
                RichText::new(format!(
                    "{} / {} delivered",
                    self.game.crates_on_goals(),
                    self.game.crates().len()
                ))
                .color(theme::GOLD),
            );
        });
        ui.add_space(8.0);
        ui.add_enabled_ui(self.dialog.is_none(), |ui| {
            ui.horizontal_wrapped(|ui| {
                let mut level = self.game.level();
                egui::ComboBox::from_id_salt("sokoban-level")
                    .selected_text(level.map_or_else(
                        || "Random level".into(),
                        |level| {
                            format!(
                                "Level {}: {}",
                                level + 1,
                                level_name(level).unwrap_or_default()
                            )
                        },
                    ))
                    .show_ui(ui, |ui| {
                        for index in 0..LEVEL_COUNT {
                            ui.selectable_value(
                                &mut level,
                                Some(index),
                                format!("{}: {}", index + 1, level_name(index).unwrap_or_default()),
                            );
                        }
                    });
                if level != self.game.level()
                    && let Some(level) = level
                {
                    self.request_start(Source::Level(level));
                }
                if ui.button("New random").clicked() {
                    let size = match self.game.source() {
                        Source::Random { size, .. } => size,
                        _ => Size::default(),
                    };
                    self.dialog = Some(Dialog::Random {
                        seed: String::new(),
                        size,
                        error: None,
                    });
                }
                if ui.button("Restart").clicked() {
                    self.dialog = Some(Dialog::Restart);
                }
                if ui
                    .add_enabled(self.game.can_undo(), egui::Button::new("Undo"))
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(self.game.can_redo(), egui::Button::new("Redo"))
                    .clicked()
                {
                    self.redo();
                }
                if ui.button("How to play").clicked() {
                    self.dialog = Some(Dialog::Help);
                }
            });
            if let Source::Random { seed, size } = self.game.source() {
                ui.label(
                    RichText::new(format!("{} · SEED {seed}", size.label()))
                        .small()
                        .color(theme::MUTED),
                );
            }
        });
    }

    fn board(&mut self, ui: &mut egui::Ui, cell: f32) {
        let size = Vec2::new(self.game.width() as f32, self.game.height() as f32) * cell;
        let (area, response) = ui.allocate_exact_size(size, Sense::click());
        self.board_rect = Some(area);
        let painter = ui.painter_at(area);
        for row in 0..self.game.height() {
            for col in 0..self.game.width() {
                let position = Position { row, col };
                let rect = Rect::from_min_size(
                    area.min + Vec2::new(col as f32, row as f32) * cell,
                    Vec2::splat(cell),
                )
                .shrink(1.0);
                match self.game.tile(position) {
                    Tile::Wall => {
                        painter.rect_filled(rect, 4.0, Color32::from_rgb(54, 83, 73));
                        painter.line_segment(
                            [
                                rect.left_bottom() - Vec2::new(0.0, 3.0),
                                rect.right_bottom() - Vec2::new(0.0, 3.0),
                            ],
                            Stroke::new(2.0, Color32::from_rgb(35, 60, 52)),
                        );
                    }
                    tile => {
                        painter.rect_filled(rect, 3.0, Color32::from_rgb(22, 78, 63));
                        if tile == Tile::Goal {
                            painter.circle_stroke(
                                rect.center(),
                                cell * 0.22,
                                Stroke::new(2.0, theme::GOLD),
                            );
                            painter.circle_filled(rect.center(), cell * 0.07, theme::GOLD);
                        }
                    }
                }
                if self.game.crates().contains(&position) {
                    let delivered = self.game.tile(position) == Tile::Goal;
                    let box_rect = rect.shrink(cell * 0.12);
                    let fill = if delivered {
                        Color32::from_rgb(114, 178, 129)
                    } else {
                        Color32::from_rgb(208, 161, 95)
                    };
                    let ink = Color32::from_rgb(75, 65, 42);
                    painter.rect_filled(box_rect, 4.0, fill);
                    painter.rect_stroke(
                        box_rect.shrink(4.0),
                        1.0,
                        Stroke::new(1.5, ink),
                        egui::StrokeKind::Inside,
                    );
                    if delivered {
                        let center = box_rect.center();
                        let elbow = center + Vec2::new(-0.03, 0.12) * cell;
                        let stroke = Stroke::new(3.0, ink);
                        painter
                            .line_segment([center + Vec2::new(-0.17, -0.02) * cell, elbow], stroke);
                        painter
                            .line_segment([elbow, center + Vec2::new(0.18, -0.15) * cell], stroke);
                    } else {
                        painter.line_segment(
                            [
                                box_rect.left_top() + Vec2::splat(5.0),
                                box_rect.right_bottom() - Vec2::splat(5.0),
                            ],
                            Stroke::new(2.0, ink),
                        );
                        painter.line_segment(
                            [
                                box_rect.right_top() + Vec2::new(-5.0, 5.0),
                                box_rect.left_bottom() + Vec2::new(5.0, -5.0),
                            ],
                            Stroke::new(2.0, ink),
                        );
                    }
                }
                if self.game.player() == position {
                    painter.circle_filled(rect.center(), cell * 0.32, theme::CREAM);
                    painter.circle_filled(
                        rect.center() + Vec2::new(-cell * 0.10, -cell * 0.04),
                        cell * 0.035,
                        theme::CHROME,
                    );
                    painter.circle_filled(
                        rect.center() + Vec2::new(cell * 0.10, -cell * 0.04),
                        cell * 0.035,
                        theme::CHROME,
                    );
                    painter.line_segment(
                        [
                            rect.center() + Vec2::new(-cell * 0.09, cell * 0.12),
                            rect.center() + Vec2::new(cell * 0.09, cell * 0.12),
                        ],
                        Stroke::new(2.0, theme::CHROME),
                    );
                }
            }
        }
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let col = ((pos.x - area.left()) / cell) as usize;
            let row = ((pos.y - area.top()) / cell) as usize;
            let player = self.game.player();
            let direction = match (
                row as isize - player.row as isize,
                col as isize - player.col as isize,
            ) {
                (-1, 0) => Some(Direction::Up),
                (1, 0) => Some(Direction::Down),
                (0, -1) => Some(Direction::Left),
                (0, 1) => Some(Direction::Right),
                _ => None,
            };
            if let Some(direction) = direction {
                self.step(direction);
            }
        }
    }

    fn direction_buttons(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(((ui.available_width() - 218.0) / 2.0).max(0.0));
            egui::Grid::new("sokoban-directions")
                .spacing(Vec2::splat(4.0))
                .show(ui, |ui| {
                    for (index, direction) in [
                        None,
                        Some(Direction::Up),
                        None,
                        Some(Direction::Left),
                        Some(Direction::Down),
                        Some(Direction::Right),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if let Some(direction) = direction {
                            let text = match direction {
                                Direction::Up => "Up",
                                Direction::Down => "Down",
                                Direction::Left => "Left",
                                Direction::Right => "Right",
                            };
                            if ui
                                .add_sized([70.0, 38.0], egui::Button::new(text))
                                .clicked()
                            {
                                self.step(direction);
                            }
                        } else {
                            ui.allocate_space(Vec2::new(70.0, 38.0));
                        }
                        if index % 3 == 2 {
                            ui.end_row();
                        }
                    }
                });
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let mut close = false;
        let mut start = None;
        let mut restart = false;
        let response = egui::Modal::new(Id::new("sokoban-dialog")).show(ctx, |ui| {
            ui.set_max_width(390.0);
            match &mut dialog {
                Dialog::Help => {
                    ui.heading("How to play Sokoban");
                    for text in [
                        "Push every wooden crate onto a gold ring. A delivered crate turns green.",
                        "Move with arrow keys, WASD, the direction buttons, or by clicking a square beside the player.",
                        "You can push one crate at a time. You cannot pull crates or push two together.",
                        "Be careful near corners: a crate can become stuck. Undo is unlimited.",
                        "Cmd / Ctrl + Z: Undo. Add Shift to Redo. Restart resets the current level.",
                        "Choose an authored level, or New random for a solvable generated warehouse. A seed recreates the same layout.",
                    ] { ui.label(text); }
                    close = ui.button("Close").clicked();
                }
                Dialog::Start(source) => {
                    ui.heading("Change level?");
                    ui.label("Your current moves and undo history will be reset.");
                    ui.horizontal(|ui| {
                        if ui.button("Start level").clicked() { start = Some(*source); }
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Restart => {
                    ui.heading("Restart this level?");
                    ui.label("Reset your moves and history, keeping the same layout.");
                    ui.horizontal(|ui| {
                        restart = ui.button("Restart").clicked();
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Random { seed, size, error } => {
                    ui.heading("Random warehouse");
                    for choice in Size::ALL { ui.radio_value(size, choice, choice.label()); }
                    ui.label("Seed (leave empty for a new random layout)");
                    ui.add(egui::TextEdit::singleline(seed).hint_text("e.g. 42"));
                    ui.label("Every layout can be solved. Starting resets your current progress.");
                    if let Some(error) = error { ui.colored_label(Color32::LIGHT_RED, error.as_str()); }
                    ui.horizontal(|ui| {
                        if ui.button("Generate").clicked() {
                            match theme::parse_seed(seed) {
                                Ok(seed) => start = Some(Source::Random { seed: seed.unwrap_or_else(crate::random_seed), size: *size }),
                                Err(reason) => *error = Some(reason.into()),
                            }
                        }
                        close = ui.button("Cancel").clicked();
                    });
                }
            }
        });
        if restart {
            self.game.restart();
            self.message = "Restarted the same layout.".into();
        } else if let Some(source) = start {
            self.start(source);
        } else if !close && !response.should_close() {
            self.dialog = Some(dialog);
        }
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.shortcuts(&ctx);
        egui::Panel::top("sokoban-toolbar")
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .inner_margin(egui::Margin::symmetric(24, 16)),
            )
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("sokoban-status")
            .frame(egui::Frame::new().fill(theme::CHROME).inner_margin(12))
            .show(ui, |ui| {
                ui.label(RichText::new(&self.message).color(theme::MUTED));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BOARD).inner_margin(16))
            .show(ui, |ui| {
                let cell = (ui.available_width() / self.game.width() as f32)
                    .min((ui.available_height() - 150.0) / self.game.height() as f32)
                    .clamp(24.0, 64.0);
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_enabled_ui(self.dialog.is_none(), |ui| {
                            self.board(ui, cell);
                            ui.add_space(10.0);
                            if self.game.is_won() {
                                ui.label(
                                    RichText::new("Level complete!")
                                        .size(24.0)
                                        .color(theme::GOLD),
                                );
                                match self.game.source() {
                                    Source::Level(level) if level + 1 < LEVEL_COUNT => {
                                        if ui.button("Next level").clicked() {
                                            self.start(Source::Level(level + 1));
                                        }
                                    }
                                    Source::Random { size, .. } => {
                                        if ui.button("New random level").clicked() {
                                            self.start(Source::Random {
                                                seed: crate::random_seed(),
                                                size,
                                            });
                                        }
                                    }
                                    _ => {
                                        ui.label("Try New random for more warehouses.");
                                    }
                                }
                            } else {
                                self.direction_buttons(ui);
                            }
                        });
                    });
                });
            });
        self.dialogs(&ctx);
    }
}

impl eframe::App for SokobanApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

#[cfg(test)]
mod tests;
