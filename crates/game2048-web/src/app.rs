use crate::theme::{self, parse_seed};
use eframe::egui::{
    self, Align2, Color32, FontId, Id, Key, Modifiers, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};
use game2048_core::{Direction, Game, SIZE, Spawn, Status, WIN_EXPONENT};

const FRAME: Color32 = Color32::from_rgb(11, 44, 37);
const EMPTY: Color32 = Color32::from_rgb(28, 82, 69);
/// Seconds for a newly spawned tile to grow to full size.
const SPAWN_SECONDS: f64 = 0.14;

enum Dialog {
    New { seed: String, error: Option<String> },
    Restart,
    Help,
}

pub struct Game2048App {
    game: Game,
    best: u64,
    message: String,
    dialog: Option<Dialog>,
    /// Keep playing after 2048 without the victory banner.
    continued: bool,
    spawned: Option<(Spawn, f64)>,
    swipe_origin: Option<Pos2>,
}

impl Game2048App {
    pub fn new(cc: &eframe::CreationContext<'_>, seed: u64) -> Self {
        theme::setup(&cc.egui_ctx);
        Self::with_game(Game::new(seed))
    }

    /// Embed a prepared core game with the same screen and controls.
    pub fn with_game(game: Game) -> Self {
        Self {
            best: game.score(),
            game,
            message: "Use the arrow keys, WASD, or swipe to slide the tiles.".into(),
            dialog: None,
            continued: false,
            spawned: None,
            swipe_origin: None,
        }
    }

    fn begin(&mut self, seed: u64) {
        self.game = Game::new(seed);
        self.continued = false;
        self.spawned = None;
        self.dialog = None;
        self.message = "New game started.".into();
    }

    fn slide(&mut self, direction: Direction, now: f64) {
        match self.game.apply(direction) {
            Ok(outcome) => {
                self.best = self.best.max(self.game.score());
                self.spawned = Some((outcome.spawned, now));
                self.message = match outcome.status {
                    Status::Over => "No moves left. Undo, or start a new game.".into(),
                    _ if outcome.gained > 0 => format!("+{}", outcome.gained),
                    _ => "Slid the tiles.".into(),
                };
            }
            Err(error) => self.message = format!("Cannot slide: {error}."),
        }
    }

    fn undo(&mut self) {
        if self.game.undo() {
            self.spawned = None;
            self.message = "Undid one move.".into();
        }
    }

    fn redo(&mut self) {
        if self.game.redo() {
            self.spawned = None;
            self.message = "Redid one move.".into();
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() {
            return;
        }
        let now = ctx.input(|i| i.time);
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
                self.slide(direction, now);
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("RUST").size(12.0).color(theme::GOLD).strong());
            ui.label(RichText::new("2048").size(25.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Best {}", self.best))
                        .size(17.0)
                        .color(theme::MUTED),
                );
                ui.separator();
                ui.label(
                    RichText::new(format!("Score {}", self.game.score()))
                        .size(17.0)
                        .color(theme::GOLD),
                );
                ui.separator();
                ui.label(format!("{} moves", self.game.moves()));
            });
        });
        ui.add_space(10.0);
        ui.add_enabled_ui(self.dialog.is_none(), |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("New game").clicked() {
                    self.dialog = Some(Dialog::New {
                        seed: String::new(),
                        error: None,
                    });
                }
                if ui.button("Restart").clicked() {
                    if self.game.moves() == 0 {
                        self.begin(self.game.seed());
                    } else {
                        self.dialog = Some(Dialog::Restart);
                    }
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
                if ui.button("How to play").clicked() {
                    self.dialog = Some(Dialog::Help);
                }
                ui.label(
                    RichText::new(format!("SEED {}", self.game.seed()))
                        .small()
                        .color(theme::MUTED),
                );
            });
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let mut close = false;
        let mut start = None;
        let response = egui::Modal::new(Id::new("game2048-dialog")).show(ctx, |ui| {
            ui.set_width(390.0);
            match &mut dialog {
                Dialog::New { seed, error } => {
                    ui.heading("New game");
                    ui.add_space(8.0);
                    ui.label("Tile seed (leave empty for a random game)");
                    ui.add(
                        egui::TextEdit::singleline(seed)
                            .hint_text("e.g. 42")
                            .desired_width(f32::INFINITY),
                    );
                    if let Some(error) = error {
                        ui.colored_label(theme::DANGER, error.as_str());
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Start game").clicked() {
                            match parse_seed(seed) {
                                Ok(seed) => start = Some(seed.unwrap_or_else(crate::random_seed)),
                                Err(reason) => *error = Some(reason.into()),
                            }
                        }
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Restart => {
                    ui.heading("Restart this game?");
                    ui.add_space(8.0);
                    ui.label("Reset your score and history, keeping the same seed.");
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Restart").clicked() {
                            start = Some(self.game.seed());
                        }
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Help => {
                    ui.heading("How to play");
                    ui.add_space(8.0);
                    for text in [
                        "Slide every tile with the arrow keys, WASD, or a swipe.",
                        "Two equal tiles that collide merge into their sum, once per move.",
                        "After each move a new 2 (or sometimes a 4) appears.",
                        "Reach the 2048 tile to win, then keep going for a higher score.",
                        "The game ends when no slide changes the board.",
                        "Cmd / Ctrl + Z: Undo. Add Shift to Redo. The same seed spawns the same tiles.",
                    ] {
                        ui.label(text);
                    }
                    ui.add_space(12.0);
                    close = ui.button("Close").clicked();
                }
            }
        });
        if let Some(seed) = start {
            self.begin(seed);
        } else if !close && !response.should_close() {
            self.dialog = Some(dialog);
        }
    }

    fn banner(&mut self, ctx: &egui::Context) {
        let status = self.game.status();
        let won = status == Status::Won && !self.continued;
        if !(won || status == Status::Over) || self.dialog.is_some() {
            return;
        }
        let mut next = false;
        egui::Modal::new(Id::new("game2048-result"))
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .corner_radius(16)
                    .stroke(Stroke::new(1.5, theme::GOLD))
                    .inner_margin(24),
            )
            .show(ctx, |ui| {
                ui.set_width(340.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(if won { "You made 2048!" } else { "Game over" })
                            .size(36.0)
                            .strong(),
                    );
                    ui.label(format!(
                        "Score {}  |  {} moves",
                        self.game.score(),
                        self.game.moves()
                    ));
                });
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if won && ui.button("Keep going").clicked() {
                        self.continued = true;
                    }
                    next = ui.button("New game").clicked();
                    if ui
                        .add_enabled(self.game.can_undo(), egui::Button::new("Undo"))
                        .clicked()
                    {
                        self.undo();
                    }
                });
            });
        if next {
            self.dialog = Some(Dialog::New {
                seed: String::new(),
                error: None,
            });
        }
    }

    /// The whole screen; `eframe::App::ui` delegates here so tests can drive it.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        self.shortcuts(&ctx);
        egui::Panel::top("game2048-toolbar")
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .inner_margin(egui::Margin::symmetric(24, 16)),
            )
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("game2048-status")
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .inner_margin(egui::Margin::symmetric(24, 12)),
            )
            .show(ui, |ui| {
                ui.label(RichText::new(&self.message).size(13.0).color(theme::MUTED));
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BOARD).inner_margin(16))
            .show(ui, |ui| {
                let side = ui
                    .available_width()
                    .min(ui.available_height())
                    .clamp(260.0, 560.0);
                ui.vertical_centered(|ui| {
                    let swipe = board(
                        ui,
                        &self.game,
                        self.spawned,
                        &mut self.swipe_origin,
                        now,
                        side,
                    );
                    if let Some(direction) = swipe.filter(|_| self.dialog.is_none()) {
                        self.slide(direction, now);
                    }
                });
            });
        self.dialogs(&ctx);
        self.banner(&ctx);
        if self
            .spawned
            .is_some_and(|(_, started)| now - started < SPAWN_SECONDS)
        {
            ctx.request_repaint();
        }
    }
}

impl eframe::App for Game2048App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

fn tile_colors(exponent: u8) -> (Color32, Color32) {
    const DARK: Color32 = Color32::from_rgb(52, 44, 34);
    const LIGHT: Color32 = Color32::from_rgb(255, 250, 239);
    match exponent {
        1 => (Color32::from_rgb(238, 228, 206), DARK),
        2 => (Color32::from_rgb(237, 220, 180), DARK),
        3 => (Color32::from_rgb(242, 177, 121), LIGHT),
        4 => (Color32::from_rgb(245, 149, 99), LIGHT),
        5 => (Color32::from_rgb(246, 124, 95), LIGHT),
        6 => (Color32::from_rgb(234, 90, 62), LIGHT),
        7 => (Color32::from_rgb(237, 207, 114), LIGHT),
        8 => (Color32::from_rgb(237, 204, 97), LIGHT),
        9 => (Color32::from_rgb(232, 196, 80), LIGHT),
        10 => (Color32::from_rgb(228, 186, 62), LIGHT),
        WIN_EXPONENT => (Color32::from_rgb(225, 176, 45), LIGHT),
        _ => (Color32::from_rgb(60, 58, 50), LIGHT),
    }
}

/// Paint the board and turn a drag across it into a swipe direction.
fn board(
    ui: &mut egui::Ui,
    game: &Game,
    spawned: Option<(Spawn, f64)>,
    swipe_origin: &mut Option<Pos2>,
    now: f64,
    side: f32,
) -> Option<Direction> {
    let (area, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::drag());
    let painter = ui.painter_at(area);
    let gap = side * 0.03;
    let cell = (side - gap * (SIZE as f32 + 1.0)) / SIZE as f32;
    painter.rect_filled(area, 10.0, FRAME);
    for (row, tiles) in game.board().iter().enumerate() {
        for (col, &exponent) in tiles.iter().enumerate() {
            let min = area.min
                + Vec2::new(
                    gap + col as f32 * (cell + gap),
                    gap + row as f32 * (cell + gap),
                );
            let mut rect = Rect::from_min_size(min, Vec2::splat(cell));
            painter.rect_filled(rect, 6.0, EMPTY);
            if exponent == 0 {
                continue;
            }
            if let Some((spawn, started)) = spawned
                && (spawn.row, spawn.col) == (row, col)
            {
                let t = ((now - started) / SPAWN_SECONDS).clamp(0.0, 1.0) as f32;
                rect = Rect::from_center_size(rect.center(), rect.size() * (0.4 + 0.6 * t));
            }
            let (fill, text) = tile_colors(exponent);
            painter.rect_filled(rect, 6.0, fill);
            let label = (1u64 << exponent).to_string();
            let scale = match label.len() {
                1 | 2 => 0.45,
                3 => 0.38,
                4 => 0.3,
                _ => 0.24,
            };
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(rect.width() * scale),
                text,
            );
        }
    }
    if response.drag_started() {
        *swipe_origin = response.interact_pointer_pos();
    }
    if !response.drag_stopped() {
        return None;
    }
    let delta = ui.input(|i| i.pointer.latest_pos())? - swipe_origin.take()?;
    if delta.length() < 24.0 {
        None
    } else if delta.x.abs() > delta.y.abs() {
        Some(if delta.x > 0.0 {
            Direction::Right
        } else {
            Direction::Left
        })
    } else {
        Some(if delta.y > 0.0 {
            Direction::Down
        } else {
            Direction::Up
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Harness {
        ctx: egui::Context,
        app: Game2048App,
        time: f64,
    }

    impl Harness {
        fn new(seed: u64) -> Self {
            let mut harness = Self {
                ctx: egui::Context::default(),
                app: Game2048App::with_game(Game::new(seed)),
                time: 0.0,
            };
            harness.frame(vec![]);
            harness
        }

        fn frame(&mut self, events: Vec<egui::Event>) {
            self.time += 0.1;
            let _output = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 800.0))),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ui| self.app.show(ui),
            );
        }

        fn key(&mut self, key: Key) {
            self.frame(vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }]);
        }

        fn swipe(&mut self, from: Pos2, to: Pos2) {
            let button = |pos, pressed| egui::Event::PointerButton {
                pos,
                pressed,
                button: egui::PointerButton::Primary,
                modifiers: Modifiers::NONE,
            };
            self.frame(vec![egui::Event::PointerMoved(from), button(from, true)]);
            self.frame(vec![egui::Event::PointerMoved((from + to.to_vec2()) / 2.0)]);
            self.frame(vec![egui::Event::PointerMoved(to)]);
            self.frame(vec![button(to, false)]);
        }
    }

    fn valid(game: &Game) -> Direction {
        Direction::ALL
            .into_iter()
            .find(|&d| game.validate(d).is_ok())
            .unwrap()
    }

    #[test]
    fn keys_slide_and_track_the_best_score() {
        let mut h = Harness::new(1);
        for _ in 0..30 {
            let key = match valid(&h.app.game) {
                Direction::Up => Key::W,
                Direction::Down => Key::ArrowDown,
                Direction::Left => Key::A,
                Direction::Right => Key::ArrowRight,
            };
            let moves = h.app.game.moves();
            h.key(key);
            assert_eq!(h.app.game.moves(), moves + 1);
        }
        assert!(h.app.best >= h.app.game.score() && h.app.best > 0);
    }

    #[test]
    fn swipes_on_the_board_slide_tiles() {
        let mut h = Harness::new(2);
        let direction = valid(&h.app.game);
        let center = Pos2::new(450.0, 420.0);
        let offset = match direction {
            Direction::Up => Vec2::new(0.0, -120.0),
            Direction::Down => Vec2::new(0.0, 120.0),
            Direction::Left => Vec2::new(-120.0, 0.0),
            Direction::Right => Vec2::new(120.0, 0.0),
        };
        h.swipe(center, center + offset);
        assert_eq!(h.app.game.moves(), 1);
    }
}
