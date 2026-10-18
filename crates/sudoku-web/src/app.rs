use crate::theme::{self, Clock, format_time, parse_seed};
use eframe::egui::{
    self, Align2, Color32, FontId, Id, Key, Modifiers, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Vec2,
};
use sudoku_core::{Action, Difficulty, Game, Position, Status};
use web_time::Duration;

const PAPER: Color32 = Color32::from_rgb(250, 246, 236);
const PEER: Color32 = Color32::from_rgb(236, 231, 215);
const SAME_DIGIT: Color32 = Color32::from_rgb(244, 228, 178);
const SELECTED: Color32 = Color32::from_rgb(235, 205, 128);
const GIVEN: Color32 = Color32::from_rgb(28, 38, 34);
const ENTERED: Color32 = Color32::from_rgb(26, 112, 92);
const NOTE: Color32 = Color32::from_rgb(105, 116, 110);
const WRONG: Color32 = Color32::from_rgb(200, 52, 44);
const THIN: Color32 = Color32::from_rgb(186, 180, 164);
const THICK: Color32 = Color32::from_rgb(36, 48, 43);

enum Dialog {
    New {
        difficulty: Difficulty,
        seed: String,
        error: Option<String>,
    },
    Restart,
    Help,
}

pub struct SudokuApp {
    game: Game,
    selected: Option<Position>,
    notes: bool,
    show_mistakes: bool,
    clock: Clock,
    message: String,
    dialog: Option<Dialog>,
    victory_dismissed: bool,
}

impl SudokuApp {
    pub fn new(cc: &eframe::CreationContext<'_>, seed: u64, difficulty: Difficulty) -> Self {
        theme::setup(&cc.egui_ctx);
        Self::with_game(Game::new(seed, difficulty))
    }

    /// Embed a prepared core game with the same screen and controls.
    pub fn with_game(game: Game) -> Self {
        Self {
            game,
            selected: Some(Position::new(4, 4)),
            notes: false,
            show_mistakes: false,
            clock: Clock::default(),
            message: "Select a cell, then type a digit or use the keypad.".into(),
            dialog: None,
            victory_dismissed: false,
        }
    }

    fn begin(&mut self, seed: u64, difficulty: Difficulty) {
        *self = Self {
            notes: self.notes,
            show_mistakes: self.show_mistakes,
            message: "New puzzle started.".into(),
            ..Self::with_game(Game::new(seed, difficulty))
        };
    }

    fn apply(&mut self, action: Action) {
        match self.game.apply(action) {
            Ok(Status::Solved) => {
                self.clock.stop();
                self.message = "Puzzle solved. Nicely done!".into();
            }
            Ok(Status::Playing) => {
                self.clock.start();
                self.message = match action {
                    Action::Set { .. } => "Placed a digit.".into(),
                    Action::Clear { .. } => "Cleared the cell.".into(),
                    Action::ToggleNote { .. } => "Updated notes.".into(),
                };
            }
            Err(error) => self.message = format!("Cannot do that: {error}."),
        }
    }

    fn digit(&mut self, digit: u8) {
        if let Some(at) = self.selected {
            self.apply(if self.notes {
                Action::ToggleNote { at, digit }
            } else {
                Action::Set { at, digit }
            });
        }
    }

    fn erase(&mut self) {
        if let Some(at) = self.selected {
            self.apply(Action::Clear { at });
        }
    }

    fn hint(&mut self) {
        if let Some(hint) = self.game.hint() {
            self.selected = Some(hint.at);
            self.apply(Action::Set {
                at: hint.at,
                digit: hint.digit,
            });
            if self.game.status() == Status::Playing {
                self.message = format!(
                    "Hint: {} goes in row {}, column {}.",
                    hint.digit,
                    hint.at.row + 1,
                    hint.at.col + 1
                );
            }
        }
    }

    fn undo(&mut self) {
        if self.game.undo() {
            self.clock.start();
            self.victory_dismissed = false;
            self.message = "Undid one move.".into();
        }
    }

    fn redo(&mut self) {
        if self.game.redo() {
            if self.game.status() == Status::Solved {
                self.clock.stop();
            }
            self.message = "Redid one move.".into();
        }
    }

    fn move_selection(&mut self, rows: isize, cols: isize) {
        let at = self.selected.unwrap_or(Position::new(4, 4));
        self.selected = Some(Position::new(
            (at.row as isize + rows).rem_euclid(9) as usize,
            (at.col as isize + cols).rem_euclid(9) as usize,
        ));
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() {
            return;
        }
        let pressed = |key| ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key));
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)) {
            self.redo();
        } else if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
            self.undo();
        }
        const DIGITS: [Key; 9] = [
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
            Key::Num9,
        ];
        for (digit, key) in (1..).zip(DIGITS) {
            if pressed(key) {
                self.digit(digit);
            }
        }
        if pressed(Key::Backspace) || pressed(Key::Delete) || pressed(Key::Num0) {
            self.erase();
        }
        for (key, rows, cols) in [
            (Key::ArrowUp, -1, 0),
            (Key::ArrowDown, 1, 0),
            (Key::ArrowLeft, 0, -1),
            (Key::ArrowRight, 0, 1),
        ] {
            if pressed(key) {
                self.move_selection(rows, cols);
            }
        }
        if pressed(Key::N) {
            self.notes = !self.notes;
        }
        if pressed(Key::H) && self.game.status() == Status::Playing {
            self.hint();
        }
        if pressed(Key::Escape) {
            self.selected = None;
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("RUST").size(10.0).color(theme::GOLD).strong());
            ui.label(RichText::new("SUDOKU").size(21.0).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} / 81", self.game.filled_count()))
                        .size(15.0)
                        .color(theme::GOLD),
                );
                ui.separator();
                ui.label(
                    RichText::new(format_time(self.clock.elapsed()))
                        .monospace()
                        .size(15.0),
                );
                ui.separator();
                ui.label(self.game.difficulty().name());
            });
        });
        ui.add_space(4.0);
        ui.add_enabled_ui(self.dialog.is_none(), |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("New game").clicked() {
                    self.dialog = Some(Dialog::New {
                        difficulty: self.game.difficulty(),
                        seed: String::new(),
                        error: None,
                    });
                }
                if ui.button("Restart").clicked() {
                    if self.game.moves() == 0 {
                        self.begin(self.game.seed(), self.game.difficulty());
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
                if ui
                    .add_enabled(
                        self.game.status() == Status::Playing,
                        egui::Button::new("Hint"),
                    )
                    .on_hover_text("H")
                    .clicked()
                {
                    self.hint();
                }
                ui.toggle_value(&mut self.notes, "Notes").on_hover_text("N");
                ui.checkbox(&mut self.show_mistakes, "Show mistakes");
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

    fn keypad(&mut self, ui: &mut egui::Ui, width: f32) {
        let gap = 6.0;
        let size = Vec2::new(((width - gap * 10.0) / 11.0).max(28.0), 46.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
            for digit in 1..=9u8 {
                let left = 9usize.saturating_sub(self.game.digit_count(digit));
                let text = RichText::new(digit.to_string()).size(20.0);
                let button = egui::Button::new(text).min_size(size);
                if ui
                    .add_enabled(left > 0 || self.notes, button)
                    .on_hover_text(format!("{left} left"))
                    .clicked()
                {
                    self.digit(digit);
                }
            }
            let erase = egui::Button::new("Erase").min_size(Vec2::new(size.x * 2.0, size.y));
            if ui.add(erase).on_hover_text("Backspace").clicked() {
                self.erase();
            }
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let mut close = false;
        let mut start = None;
        let response = egui::Modal::new(Id::new("sudoku-dialog")).show(ctx, |ui| {
            ui.set_width(390.0);
            match &mut dialog {
                Dialog::New {
                    difficulty,
                    seed,
                    error,
                } => {
                    ui.heading("New game");
                    ui.add_space(8.0);
                    ui.label("Difficulty");
                    ui.horizontal(|ui| {
                        for option in Difficulty::ALL {
                            ui.selectable_value(difficulty, option, option.name());
                        }
                    });
                    ui.add_space(6.0);
                    ui.label("Puzzle seed (leave empty for a random puzzle)");
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
                                    start =
                                        Some((seed.unwrap_or_else(crate::random_seed), *difficulty))
                                }
                                Err(reason) => *error = Some(reason.into()),
                            }
                        }
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Restart => {
                    ui.heading("Restart this puzzle?");
                    ui.add_space(8.0);
                    ui.label("Clear your entries and history, keeping the same puzzle.");
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("Restart").clicked() {
                            start = Some((self.game.seed(), self.game.difficulty()));
                        }
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Help => {
                    ui.heading("How to play");
                    ui.add_space(8.0);
                    for text in [
                        "Fill every row, column, and 3×3 box with the digits 1 to 9.",
                        "Every puzzle has exactly one solution. Dark digits are fixed.",
                        "Click a cell or use the arrow keys, then type 1–9. Backspace erases.",
                        "Press N or Notes to pencil in candidates instead of answers.",
                        "Placing a digit removes it from notes in the same row, column, and box.",
                        "Repeated digits turn red. Show mistakes also checks against the solution.",
                        "Cmd / Ctrl + Z: Undo. Add Shift to Redo. H: Hint.",
                    ] {
                        ui.label(text);
                    }
                    ui.add_space(12.0);
                    close = ui.button("Close").clicked();
                }
            }
        });
        if let Some((seed, difficulty)) = start {
            self.begin(seed, difficulty);
        } else if !close && !response.should_close() {
            self.dialog = Some(dialog);
        }
    }

    fn victory(&mut self, ctx: &egui::Context) {
        if self.game.status() != Status::Solved || self.victory_dismissed || self.dialog.is_some() {
            return;
        }
        let mut next = false;
        egui::Modal::new(Id::new("sudoku-victory"))
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .corner_radius(16)
                    .stroke(Stroke::new(1.5, theme::GOLD))
                    .inner_margin(24),
            )
            .show(ctx, |ui| {
                ui.set_width(360.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("P U Z Z L E   S O L V E D")
                            .size(13.0)
                            .color(theme::GOLD),
                    );
                    ui.label(RichText::new("Well done!").size(42.0).strong());
                    ui.label(format!(
                        "{}  |  {}  |  {} moves",
                        self.game.difficulty().name(),
                        format_time(self.clock.elapsed()),
                        self.game.moves()
                    ));
                });
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    next = ui.button("New game").clicked();
                    self.victory_dismissed = ui.button("View board").clicked();
                });
            });
        if next {
            self.dialog = Some(Dialog::New {
                difficulty: self.game.difficulty(),
                seed: String::new(),
                error: None,
            });
        }
    }

    /// The whole screen; `eframe::App::ui` delegates here so tests can drive it.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.shortcuts(&ctx);
        egui::Panel::top("sudoku-toolbar")
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("sudoku-status")
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
                let side = (ui.available_width())
                    .min(ui.available_height() - 72.0)
                    .clamp(270.0, 620.0);
                let enabled = self.dialog.is_none();
                ui.add_enabled_ui(enabled, |ui| {
                    ui.vertical_centered(|ui| {
                        if let Some(at) =
                            grid(ui, &self.game, self.selected, self.show_mistakes, side)
                        {
                            self.selected = Some(at);
                        }
                    });
                    ui.add_space(12.0);
                    if self.game.status() == Status::Playing {
                        self.keypad(ui, side);
                    }
                });
            });
        self.dialogs(&ctx);
        self.victory(&ctx);
        if self.clock.is_running() {
            ctx.request_repaint_after(Duration::from_secs(1));
        }
    }
}

impl eframe::App for SudokuApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

/// Paint the grid and return the cell under a click.
fn grid(
    ui: &mut egui::Ui,
    game: &Game,
    selected: Option<Position>,
    show_mistakes: bool,
    side: f32,
) -> Option<Position> {
    let (area, response) = ui.allocate_exact_size(Vec2::splat(side), Sense::click());
    let painter = ui.painter_at(area);
    let cell = side / 9.0;
    let rect_of = |at: Position| {
        Rect::from_min_size(
            area.min + Vec2::new(at.col as f32 * cell, at.row as f32 * cell),
            Vec2::splat(cell),
        )
    };
    let mistakes = if show_mistakes {
        game.mistakes()
    } else {
        Vec::new()
    };
    let selected_digit = selected.and_then(|at| game.value(at));
    painter.rect_filled(area, 6.0, PAPER);
    for row in 0..9 {
        for col in 0..9 {
            let at = Position::new(row, col);
            let rect = rect_of(at);
            let value = game.value(at);
            let fill = match selected {
                Some(s) if s == at => Some(SELECTED),
                _ if value.is_some() && value == selected_digit => Some(SAME_DIGIT),
                Some(s)
                    if s.row == row
                        || s.col == col
                        || (s.row / 3, s.col / 3) == (row / 3, col / 3) =>
                {
                    Some(PEER)
                }
                _ => None,
            };
            if let Some(fill) = fill {
                painter.rect_filled(rect, 0.0, fill);
            }
            if let Some(digit) = value {
                let color = if game.conflicts(at) || mistakes.contains(&at) {
                    WRONG
                } else if game.is_given(at) {
                    GIVEN
                } else {
                    ENTERED
                };
                let font = if game.is_given(at) {
                    FontId::proportional(cell * 0.6)
                } else {
                    FontId::proportional(cell * 0.56)
                };
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    digit.to_string(),
                    font,
                    color,
                );
            } else {
                for digit in (1..=9).filter(|&d| game.has_note(at, d)) {
                    let k = usize::from(digit - 1);
                    let center = rect.min
                        + Vec2::new(
                            (k % 3) as f32 * cell / 3.0 + cell / 6.0,
                            (k / 3) as f32 * cell / 3.0 + cell / 6.0,
                        );
                    painter.text(
                        center,
                        Align2::CENTER_CENTER,
                        digit.to_string(),
                        FontId::proportional(cell * 0.24),
                        NOTE,
                    );
                }
            }
        }
    }
    for line in 1..9 {
        let offset = line as f32 * cell;
        let stroke = if line % 3 == 0 {
            Stroke::new(2.2, THICK)
        } else {
            Stroke::new(1.0, THIN)
        };
        painter.line_segment(
            [
                area.min + Vec2::new(offset, 0.0),
                Pos2::new(area.min.x + offset, area.max.y),
            ],
            stroke,
        );
        painter.line_segment(
            [
                area.min + Vec2::new(0.0, offset),
                Pos2::new(area.max.x, area.min.y + offset),
            ],
            stroke,
        );
    }
    painter.rect_stroke(area, 6.0, Stroke::new(2.6, THICK), StrokeKind::Inside);
    let pos = response
        .interact_pointer_pos()
        .filter(|_| response.clicked())?;
    let local = pos - area.min;
    Some(Position::new(
        ((local.y / cell) as usize).min(8),
        ((local.x / cell) as usize).min(8),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Harness {
        ctx: egui::Context,
        app: SudokuApp,
        time: f64,
    }

    impl Harness {
        fn new() -> Self {
            let mut harness = Self {
                ctx: egui::Context::default(),
                app: SudokuApp::with_game(Game::new(42, Difficulty::Easy)),
                time: 0.0,
            };
            harness.frame(vec![]);
            harness
        }

        fn frame(&mut self, events: Vec<egui::Event>) {
            self.time += 0.1;
            let _output = self.ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 900.0))),
                    time: Some(self.time),
                    events,
                    ..Default::default()
                },
                |ui| self.app.show(ui),
            );
        }

        fn key(&mut self, key: Key, modifiers: Modifiers) {
            self.frame(vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }]);
        }
    }

    fn empty_cell(game: &Game) -> Position {
        (0..81)
            .map(|i| Position::new(i / 9, i % 9))
            .find(|&at| game.value(at).is_none())
            .unwrap()
    }

    #[test]
    fn keyboard_entry_notes_and_undo() {
        let mut h = Harness::new();
        let at = empty_cell(&h.app.game);
        h.app.selected = Some(at);
        h.key(Key::Num5, Modifiers::NONE);
        assert_eq!(h.app.game.value(at), Some(5));
        h.key(Key::Backspace, Modifiers::NONE);
        assert_eq!(h.app.game.value(at), None);
        h.key(Key::N, Modifiers::NONE);
        h.key(Key::Num3, Modifiers::NONE);
        assert!(h.app.game.has_note(at, 3));
        h.key(Key::Z, Modifiers::COMMAND);
        assert!(!h.app.game.has_note(at, 3));
        h.key(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
        assert!(h.app.game.has_note(at, 3));
    }

    #[test]
    fn arrows_wrap_around_the_grid() {
        let mut h = Harness::new();
        h.app.selected = Some(Position::new(0, 0));
        h.key(Key::ArrowUp, Modifiers::NONE);
        h.key(Key::ArrowLeft, Modifiers::NONE);
        assert_eq!(h.app.selected, Some(Position::new(8, 8)));
    }

    #[test]
    fn hints_finish_the_puzzle_and_stop_the_clock() {
        let mut h = Harness::new();
        while h.app.game.status() == Status::Playing {
            h.key(Key::H, Modifiers::NONE);
        }
        assert!(!h.app.clock.is_running());
        assert!(h.app.game.mistakes().is_empty());
    }
}
