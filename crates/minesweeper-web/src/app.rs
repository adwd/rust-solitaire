use crate::theme::{self, Clock, format_time, parse_seed};
use eframe::egui::{
    self, Align2, Color32, FontId, Id, Key, Modifiers, Painter, Pos2, Rect, RichText, Sense, Shape,
    Stroke, StrokeKind, Vec2,
};
use minesweeper_core::{Action, CellView, Config, Game, MoveError, Position, Status};
use web_time::Duration;

const COVER: Color32 = Color32::from_rgb(62, 118, 97);
const COVER_HOVER: Color32 = Color32::from_rgb(78, 136, 112);
const OPEN: Color32 = Color32::from_rgb(22, 54, 46);
const GRID: Color32 = Color32::from_rgb(12, 38, 32);
const FLAG: Color32 = Color32::from_rgb(236, 84, 70);
const MINE: Color32 = Color32::from_rgb(18, 20, 20);
const BLAST: Color32 = Color32::from_rgb(196, 58, 46);

const PRESETS: [(&str, Config); 3] = [
    ("Beginner", Config::BEGINNER),
    ("Intermediate", Config::INTERMEDIATE),
    ("Expert", Config::EXPERT),
];

enum Dialog {
    New {
        config: Config,
        seed: String,
        error: Option<String>,
    },
    Help,
}

pub struct MinesweeperApp {
    game: Game,
    flag_mode: bool,
    clock: Clock,
    message: String,
    dialog: Option<Dialog>,
    result_dismissed: bool,
}

impl MinesweeperApp {
    pub fn new(cc: &eframe::CreationContext<'_>, seed: u64, config: Config) -> Self {
        theme::setup(&cc.egui_ctx);
        Self::with_game(Game::new(seed, config))
    }

    /// Embed a prepared core game with the same screen and controls.
    pub fn with_game(game: Game) -> Self {
        Self {
            game,
            flag_mode: false,
            clock: Clock::default(),
            message: "Click to reveal. Right-click or long-press to flag. The first click is safe."
                .into(),
            dialog: None,
            result_dismissed: false,
        }
    }

    fn begin(&mut self, seed: u64, config: Config) {
        *self = Self {
            flag_mode: self.flag_mode,
            message: "New game started.".into(),
            ..Self::with_game(Game::new(seed, config))
        };
    }

    fn apply(&mut self, action: Action) {
        match self.game.apply(action) {
            Ok(Status::Won) => {
                self.clock.stop();
                self.message = "Board cleared!".into();
            }
            Ok(Status::Lost) => {
                self.clock.stop();
                self.message = "Boom. Start a new game to try again.".into();
            }
            Ok(_) => {
                if !matches!(action, Action::ToggleFlag(_)) {
                    self.clock.start();
                }
                self.message = format!("{} mines left.", self.game.mines_left());
            }
            Err(MoveError::Revealed) => {}
            Err(error) => self.message = format!("Cannot do that: {error}."),
        }
    }

    /// A primary click: reveal, chord on a number, or flag in flag mode.
    fn press(&mut self, at: Position) {
        let action = match self.game.cell(at) {
            CellView::Revealed(n) if n > 0 => Action::Chord(at),
            CellView::Hidden | CellView::Flagged if self.flag_mode => Action::ToggleFlag(at),
            _ => Action::Reveal(at),
        };
        self.apply(action);
    }

    fn preset_name(config: Config) -> String {
        PRESETS
            .iter()
            .find(|(_, preset)| *preset == config)
            .map_or_else(
                || format!("{}×{}", config.cols, config.rows),
                |(name, _)| (*name).into(),
            )
    }

    fn new_dialog(&mut self) {
        self.dialog = Some(Dialog::New {
            config: self.game.config(),
            seed: String::new(),
            error: None,
        });
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F)) {
            self.flag_mode = !self.flag_mode;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::R)) {
            self.begin(crate::random_seed(), self.game.config());
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("RUST").size(12.0).color(theme::GOLD).strong());
            ui.label(RichText::new("MINESWEEPER").size(25.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} mines", self.game.mines_left()))
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
                ui.label(Self::preset_name(self.game.config()));
            });
        });
        ui.add_space(10.0);
        ui.add_enabled_ui(self.dialog.is_none(), |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("New game").clicked() {
                    self.new_dialog();
                }
                if ui.button("Quick restart").on_hover_text("R").clicked() {
                    self.begin(crate::random_seed(), self.game.config());
                }
                ui.separator();
                ui.toggle_value(&mut self.flag_mode, "Flag mode")
                    .on_hover_text("F: clicks place flags instead of revealing");
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
        let response = egui::Modal::new(Id::new("minesweeper-dialog")).show(ctx, |ui| {
            ui.set_width(390.0);
            match &mut dialog {
                Dialog::New {
                    config,
                    seed,
                    error,
                } => {
                    ui.heading("New game");
                    ui.add_space(8.0);
                    ui.label("Board");
                    ui.horizontal(|ui| {
                        for (name, preset) in PRESETS {
                            ui.selectable_value(config, preset, name);
                        }
                    });
                    ui.label(
                        RichText::new(format!(
                            "{} × {} with {} mines",
                            config.cols, config.rows, config.mines
                        ))
                        .small()
                        .color(theme::MUTED),
                    );
                    ui.add_space(6.0);
                    ui.label("Mine seed (leave empty for a random board)");
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
                                Ok(seed) => {
                                    start = Some((seed.unwrap_or_else(crate::random_seed), *config))
                                }
                                Err(reason) => *error = Some(reason.into()),
                            }
                        }
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Help => {
                    ui.heading("How to play");
                    ui.add_space(8.0);
                    for text in [
                        "Reveal every cell that does not hide a mine.",
                        "A number counts the mines in the eight surrounding cells.",
                        "The first cell you reveal, and its neighbors when possible, are always safe.",
                        "Right-click or long-press to flag a suspected mine. F toggles flag mode.",
                        "Click a number whose flags are all placed to reveal its other neighbors.",
                        "R starts a fresh board of the same size. The same seed and first click give the same mines.",
                    ] {
                        ui.label(text);
                    }
                    ui.add_space(12.0);
                    close = ui.button("Close").clicked();
                }
            }
        });
        if let Some((seed, config)) = start {
            self.begin(seed, config);
        } else if !close && !response.should_close() {
            self.dialog = Some(dialog);
        }
    }

    fn result(&mut self, ctx: &egui::Context) {
        let status = self.game.status();
        if !matches!(status, Status::Won | Status::Lost)
            || self.result_dismissed
            || self.dialog.is_some()
        {
            return;
        }
        let won = status == Status::Won;
        let mut again = false;
        egui::Modal::new(Id::new("minesweeper-result"))
            .backdrop_color(Color32::from_black_alpha(45))
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .corner_radius(16)
                    .stroke(Stroke::new(1.5, if won { theme::GOLD } else { FLAG }))
                    .inner_margin(24),
            )
            .show(ctx, |ui| {
                ui.set_width(340.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(if won { "Cleared!" } else { "Boom!" })
                            .size(40.0)
                            .strong(),
                    );
                    ui.label(format!(
                        "{}  |  {}  |  {} moves",
                        Self::preset_name(self.game.config()),
                        format_time(self.clock.elapsed()),
                        self.game.moves()
                    ));
                });
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    again = ui.button("Play again").clicked();
                    if ui.button("New game").clicked() {
                        self.new_dialog();
                    }
                    self.result_dismissed = ui.button("View board").clicked();
                });
            });
        if again {
            self.begin(crate::random_seed(), self.game.config());
        }
    }

    /// The whole screen; `eframe::App::ui` delegates here so tests can drive it.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.shortcuts(&ctx);
        egui::Panel::top("minesweeper-toolbar")
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .inner_margin(egui::Margin::symmetric(24, 16)),
            )
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("minesweeper-status")
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
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            let enabled = self.dialog.is_none();
                            let (_, press) =
                                ui.add_enabled_ui(enabled, |ui| board(ui, &self.game)).inner;
                            match press {
                                Some(Press::Primary(at)) => self.press(at),
                                Some(Press::Secondary(at)) => self.apply(Action::ToggleFlag(at)),
                                None => {}
                            }
                        });
                    });
            });
        self.dialogs(&ctx);
        self.result(&ctx);
        if self.clock.is_running() {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

impl eframe::App for MinesweeperApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

enum Press {
    Primary(Position),
    Secondary(Position),
}

fn number_color(n: u8) -> Color32 {
    match n {
        1 => Color32::from_rgb(125, 186, 255),
        2 => Color32::from_rgb(134, 222, 140),
        3 => Color32::from_rgb(255, 124, 112),
        4 => Color32::from_rgb(196, 156, 255),
        5 => Color32::from_rgb(255, 176, 92),
        6 => Color32::from_rgb(96, 222, 222),
        7 => theme::CREAM,
        _ => theme::MUTED,
    }
}

fn flag(painter: &Painter, rect: Rect, color: Color32) {
    let s = rect.width();
    let p = |x: f32, y: f32| rect.min + Vec2::new(x * s, y * s);
    painter.line_segment(
        [p(0.38, 0.2), p(0.38, 0.8)],
        Stroke::new(s * 0.07, theme::CREAM),
    );
    painter.add(Shape::convex_polygon(
        vec![p(0.4, 0.2), p(0.76, 0.36), p(0.4, 0.52)],
        color,
        Stroke::NONE,
    ));
    painter.line_segment(
        [p(0.24, 0.8), p(0.56, 0.8)],
        Stroke::new(s * 0.07, theme::CREAM),
    );
}

fn mine(painter: &Painter, rect: Rect) {
    let center = rect.center();
    let r = rect.width() * 0.24;
    for k in 0..4 {
        let angle = k as f32 * std::f32::consts::FRAC_PI_4;
        let offset = Vec2::angled(angle) * r * 1.45;
        painter.line_segment(
            [center - offset, center + offset],
            Stroke::new(r * 0.32, MINE),
        );
    }
    painter.circle_filled(center, r, MINE);
    painter.circle_filled(
        center - Vec2::splat(r * 0.35),
        r * 0.25,
        Color32::from_gray(200),
    );
}

/// Paint the board and return its area with the cell under a click.
fn board(ui: &mut egui::Ui, game: &Game) -> (Rect, Option<Press>) {
    let config = game.config();
    let cell = (ui.available_width() / config.cols as f32)
        .min(ui.available_height() / config.rows as f32)
        .clamp(20.0, 40.0);
    let size = Vec2::new(config.cols as f32, config.rows as f32) * cell;
    let (area, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter_at(area);
    let at_pos = |pos: Pos2| {
        let local = pos - area.min;
        Position::new(
            ((local.y / cell) as usize).min(config.rows - 1),
            ((local.x / cell) as usize).min(config.cols - 1),
        )
    };
    let hovered = response.hover_pos().map(at_pos);
    painter.rect_filled(area, 4.0, GRID);
    for row in 0..config.rows {
        for col in 0..config.cols {
            let at = Position::new(row, col);
            let rect = Rect::from_min_size(
                area.min + Vec2::new(col as f32, row as f32) * cell,
                Vec2::splat(cell),
            )
            .shrink(1.0);
            let view = game.cell(at);
            let fill = match view {
                CellView::Hidden | CellView::Flagged if hovered == Some(at) => COVER_HOVER,
                CellView::Hidden | CellView::Flagged => COVER,
                CellView::Exploded => BLAST,
                _ => OPEN,
            };
            painter.rect_filled(rect, 3.0, fill);
            match view {
                CellView::Hidden => {}
                CellView::Flagged => flag(&painter, rect, FLAG),
                CellView::Revealed(0) => {}
                CellView::Revealed(n) => {
                    painter.text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        n.to_string(),
                        FontId::monospace(cell * 0.58),
                        number_color(n),
                    );
                }
                CellView::Mine | CellView::Exploded => mine(&painter, rect),
                CellView::WrongFlag => {
                    flag(&painter, rect, theme::MUTED);
                    painter.line_segment(
                        [rect.left_top(), rect.right_bottom()],
                        Stroke::new(2.0, FLAG),
                    );
                }
            }
        }
    }
    painter.rect_stroke(area, 4.0, Stroke::new(2.0, GRID), StrokeKind::Outside);
    let pos = response.interact_pointer_pos().or(response.hover_pos());
    let press = if response.secondary_clicked() || response.long_touched() {
        pos.map(|pos| Press::Secondary(at_pos(pos)))
    } else if response.clicked() {
        pos.map(|pos| Press::Primary(at_pos(pos)))
    } else {
        None
    };
    (area, press)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Harness {
        ctx: egui::Context,
        app: MinesweeperApp,
        time: f64,
        board: Rect,
    }

    impl Harness {
        fn new(seed: u64) -> Self {
            let mut harness = Self {
                ctx: egui::Context::default(),
                app: MinesweeperApp::with_game(Game::new(seed, Config::BEGINNER)),
                time: 0.0,
                board: Rect::NOTHING,
            };
            harness.frame(vec![]);
            harness
        }

        /// Drive only the board, like the central panel does.
        fn frame(&mut self, events: Vec<egui::Event>) {
            self.time += 0.1;
            let mut result = None;
            let _output = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 800.0))),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ui| result = Some(board(ui, &self.app.game)),
            );
            let (area, press) = result.unwrap();
            self.board = area;
            match press {
                Some(Press::Primary(at)) => self.app.press(at),
                Some(Press::Secondary(at)) => self.app.apply(Action::ToggleFlag(at)),
                None => {}
            }
        }

        fn click(&mut self, at: Position, button: egui::PointerButton) {
            let cell = self.board.width() / self.app.game.config().cols as f32;
            let pos = self.board.min + Vec2::new(at.col as f32 + 0.5, at.row as f32 + 0.5) * cell;
            let event = |pressed| egui::Event::PointerButton {
                pos,
                pressed,
                button,
                modifiers: Modifiers::NONE,
            };
            self.frame(vec![egui::Event::PointerMoved(pos), event(true)]);
            self.frame(vec![event(false)]);
            self.frame(vec![]);
        }
    }

    #[test]
    fn clicks_reveal_flag_and_start_the_clock() {
        let mut h = Harness::new(4);
        let corner = Position::new(0, 8);
        h.click(corner, egui::PointerButton::Secondary);
        assert_eq!(h.app.game.cell(corner), CellView::Flagged);
        assert!(
            !h.app.clock.is_running(),
            "flags alone do not start the clock"
        );
        h.click(corner, egui::PointerButton::Secondary);
        h.click(Position::new(4, 4), egui::PointerButton::Primary);
        assert_eq!(h.app.game.cell(Position::new(4, 4)), CellView::Revealed(0));
        assert!(h.app.clock.is_running());
    }

    #[test]
    fn flag_mode_turns_primary_clicks_into_flags() {
        let mut h = Harness::new(5);
        h.app.flag_mode = true;
        h.click(Position::new(2, 3), egui::PointerButton::Primary);
        assert_eq!(h.app.game.cell(Position::new(2, 3)), CellView::Flagged);
        assert_eq!(h.app.game.status(), Status::Ready);
    }
}
