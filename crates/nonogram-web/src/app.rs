use crate::theme;
use eframe::egui::{
    self, Align2, Color32, FontId, Id, Key, Modifiers, Rect, RichText, Sense, Stroke, Vec2,
};
use nonogram_core::{Cell, Game, PUZZLE_COUNT, Size, Source, clue_span, puzzle_size};

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

/// A pointer stroke that paints the same mark on every square it crosses.
/// Nothing is written to the game until the pointer is released, so the whole
/// drag becomes one undo step.
struct DragStroke {
    start: (usize, usize),
    value: Cell,
    cells: Vec<(usize, usize)>,
    last: egui::Pos2,
    button: egui::PointerButton,
}

pub struct NonogramApp {
    game: Game,
    selected: (usize, usize),
    mode: Cell,
    dialog: Option<Dialog>,
    board_rect: Option<Rect>,
    stroke: Option<DragStroke>,
    show_spans: bool,
}

impl NonogramApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::setup(&cc.egui_ctx);
        Self::with_game(Game::default())
    }

    pub fn with_game(game: Game) -> Self {
        Self {
            game,
            selected: (0, 0),
            mode: Cell::Filled,
            dialog: None,
            board_rect: None,
            stroke: None,
            show_spans: false,
        }
    }

    fn start(&mut self, source: Source) {
        let game = match source {
            Source::Puzzle(puzzle) => Game::new(puzzle),
            Source::Random { seed, size } => Some(Game::random(seed, size)),
        };
        if let Some(game) = game {
            self.game = game;
            self.selected = (0, 0);
            self.mode = Cell::Filled;
            self.dialog = None;
            self.stroke = None;
        }
    }

    fn request_start(&mut self, source: Source) {
        if self.game.moves() > 0 && !self.game.is_won() {
            self.dialog = Some(Dialog::Start(source));
        } else {
            self.start(source);
        }
    }

    fn mark(&mut self, row: usize, col: usize, mode: Cell) {
        let value = if self.game.cell(row, col) == Some(mode) {
            Cell::Unknown
        } else {
            mode
        };
        self.game.apply(row, col, value);
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() || egui::Popup::is_any_open(ctx) {
            return;
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)) {
            self.game.redo();
        } else if ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Z)) {
            self.game.undo();
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::L)) {
            self.show_spans = !self.show_spans;
        }
        for (key, dr, dc) in [
            (Key::ArrowUp, -1, 0),
            (Key::ArrowDown, 1, 0),
            (Key::ArrowLeft, 0, -1),
            (Key::ArrowRight, 0, 1),
        ] {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                self.selected.0 = self
                    .selected
                    .0
                    .saturating_add_signed(dr)
                    .min(self.game.size() - 1);
                self.selected.1 = self
                    .selected
                    .1
                    .saturating_add_signed(dc)
                    .min(self.game.size() - 1);
            }
        }
        for (key, mode) in [
            (Key::Space, Cell::Filled),
            (Key::X, Cell::Crossed),
            (Key::Backspace, Cell::Unknown),
            (Key::Delete, Cell::Unknown),
        ] {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) {
                self.mark(self.selected.0, self.selected.1, mode);
            }
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 6.0);
        ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
        ui.add_enabled_ui(self.dialog.is_none(), |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("RUST").size(10.0).color(theme::GOLD).strong());
                ui.label(RichText::new("Nonogram").size(21.0).strong());
                ui.separator();
                let mut puzzle = self.game.puzzle();
                egui::ComboBox::from_id_salt("nonogram-puzzle")
                    .selected_text(puzzle.map_or_else(
                        || format!("Random · {}×{}", self.game.size(), self.game.size()),
                        |puzzle| {
                            format!(
                                "Puzzle {} · {}×{}",
                                puzzle + 1,
                                self.game.size(),
                                self.game.size()
                            )
                        },
                    ))
                    .show_ui(ui, |ui| {
                        for index in 0..PUZZLE_COUNT {
                            let size = puzzle_size(index).unwrap_or_default();
                            ui.selectable_value(
                                &mut puzzle,
                                Some(index),
                                format!("Puzzle {} · {size}×{size}", index + 1),
                            );
                        }
                    });
                if puzzle != self.game.puzzle()
                    && let Some(puzzle) = puzzle
                {
                    self.request_start(Source::Puzzle(puzzle));
                }
                if ui.button("New random").clicked() {
                    let size = match self.game.size() {
                        15 => Size::ExtraLarge,
                        10 => Size::Large,
                        _ => Size::Small,
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
                    self.game.undo();
                }
                if ui
                    .add_enabled(self.game.can_redo(), egui::Button::new("Redo"))
                    .clicked()
                {
                    self.game.redo();
                }
                ui.toggle_value(&mut self.show_spans, "Hint").on_hover_text(
                    "Line spans: filled squares plus the gaps between runs. \
                     A gold value equal to the board size means the line is forced.",
                );
                if ui.button("Help").clicked() {
                    self.dialog = Some(Dialog::Help);
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label("Mark");
                ui.selectable_value(&mut self.mode, Cell::Filled, "■ Fill");
                ui.selectable_value(&mut self.mode, Cell::Crossed, "× Cross");
                ui.selectable_value(&mut self.mode, Cell::Unknown, "Erase");
                ui.separator();
                ui.label(
                    RichText::new(format!(
                        "{}×{} · {} moves",
                        self.game.size(),
                        self.game.size(),
                        self.game.moves()
                    ))
                    .small()
                    .color(theme::MUTED),
                );
                if let Source::Random { seed, .. } = self.game.source() {
                    ui.label(
                        RichText::new(format!("SEED {seed}"))
                            .small()
                            .color(theme::MUTED),
                    );
                }
            });
        });
    }

    fn cell_at(pos: egui::Pos2, grid: Rect, cell: f32, n: usize) -> Option<(usize, usize)> {
        if !grid.contains(pos) {
            return None;
        }
        let col = ((pos.x - grid.left()) / cell).floor() as usize;
        let row = ((pos.y - grid.top()) / cell).floor() as usize;
        (row < n && col < n).then_some((row, col))
    }

    fn begin_stroke(
        &mut self,
        pos: egui::Pos2,
        grid: Rect,
        cell: f32,
        button: egui::PointerButton,
    ) {
        let Some(start) = Self::cell_at(pos, grid, cell, self.game.size()) else {
            return;
        };
        let mode = if button == egui::PointerButton::Secondary {
            Cell::Crossed
        } else {
            self.mode
        };
        // Starting on a square that already has this mark erases instead, so
        // dragging over a filled run clears the whole run.
        let value = if self.game.cell(start.0, start.1) == Some(mode) {
            Cell::Unknown
        } else {
            mode
        };
        self.selected = start;
        self.stroke = Some(DragStroke {
            start,
            value,
            cells: vec![start],
            last: pos,
            button,
        });
    }

    /// Adds every square the pointer passed over to the stroke. Sampling along
    /// the segment keeps fast drags from skipping squares between frames.
    fn extend_stroke(&mut self, pos: egui::Pos2, grid: Rect, cell: f32) {
        let n = self.game.size();
        let Some(stroke) = &mut self.stroke else {
            return;
        };
        let from = stroke.last;
        let steps = (((pos - from).length() / (cell * 0.3)).ceil() as usize).max(1);
        let mut reached = None;
        for step in 0..=steps {
            let sample = from + (pos - from) * (step as f32 / steps as f32);
            let Some(square) = Self::cell_at(sample, grid, cell, n) else {
                continue;
            };
            if !stroke.cells.contains(&square) {
                stroke.cells.push(square);
            }
            reached = Some(square);
        }
        stroke.last = pos;
        if let Some(square) = reached {
            self.selected = square;
        }
    }

    fn finish_stroke(&mut self) {
        let Some(stroke) = self.stroke.take() else {
            return;
        };
        let edits: Vec<_> = stroke
            .cells
            .into_iter()
            .filter(|&(row, col)| self.game.cell(row, col) != Some(stroke.value))
            .map(|(row, col)| (row, col, stroke.value))
            .collect();
        self.game.apply_batch(edits);
    }

    fn pointer(&mut self, ui: &egui::Ui, response: &egui::Response, grid: Rect, cell: f32) {
        if !response.enabled() || self.game.is_won() {
            self.stroke = None;
            return;
        }
        let (
            primary_pressed,
            secondary_pressed,
            primary_released,
            secondary_released,
            any_down,
            pos,
        ) = ui.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.secondary_pressed(),
                i.pointer.primary_released(),
                i.pointer.secondary_released(),
                i.pointer.any_down(),
                i.pointer.interact_pos(),
            )
        });
        if self.stroke.is_none() {
            let button = if primary_pressed {
                Some(egui::PointerButton::Primary)
            } else if secondary_pressed {
                Some(egui::PointerButton::Secondary)
            } else {
                None
            };
            if let (Some(button), Some(pos)) = (button, pos) {
                self.begin_stroke(pos, grid, cell, button);
            }
        }
        // A long touch still places a cross, matching the old secondary click.
        if response.long_touched()
            && let Some(stroke) = &mut self.stroke
        {
            stroke.value = if self.game.cell(stroke.start.0, stroke.start.1) == Some(Cell::Crossed)
            {
                Cell::Unknown
            } else {
                Cell::Crossed
            };
        }
        let Some(button) = self.stroke.as_ref().map(|stroke| stroke.button) else {
            return;
        };
        if let Some(pos) = pos {
            self.extend_stroke(pos, grid, cell);
        }
        let released = match button {
            egui::PointerButton::Primary => primary_released,
            egui::PointerButton::Secondary => secondary_released,
            _ => false,
        };
        if released || !any_down {
            self.finish_stroke();
        }
    }

    fn displayed_cell(&self, row: usize, col: usize) -> Cell {
        if let Some(stroke) = &self.stroke
            && stroke.cells.contains(&(row, col))
        {
            stroke.value
        } else {
            self.game.cell(row, col).unwrap_or_default()
        }
    }

    fn clue_space(&self) -> (f32, f32) {
        let extra = if self.show_spans { 0.75 } else { 0.0 };
        let rows = self
            .game
            .row_clues()
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(1) as f32;
        let columns = self
            .game
            .column_clues()
            .iter()
            .map(Vec::len)
            .max()
            .unwrap_or(1) as f32;
        (rows + 0.5 + extra, columns + 0.5 + extra)
    }

    fn board(&mut self, ui: &mut egui::Ui, cell: f32) {
        let (left, top) = self.clue_space();
        let n = self.game.size();
        let (area, _) = ui.allocate_exact_size(
            Vec2::new(n as f32 + left, n as f32 + top) * cell,
            Sense::hover(),
        );
        let grid = Rect::from_min_size(
            area.min + Vec2::new(left, top) * cell,
            Vec2::splat(n as f32 * cell),
        );
        self.board_rect = Some(grid);
        let response = ui.interact(grid, Id::new("nonogram-grid"), Sense::click_and_drag());
        self.pointer(ui, &response, grid, cell);
        let painter = ui.painter_at(area.expand(2.0));
        let font = FontId::proportional(cell * 0.44);
        let span_font = FontId::proportional(cell * 0.36);
        let span_x = -(left - 0.7);
        let span_y = -(top - 0.7);
        for row in 0..n {
            let clue = &self.game.row_clues()[row];
            let color = if self.game.row_complete(row) {
                theme::MUTED
            } else {
                theme::CREAM
            };
            for (index, value) in clue.iter().rev().enumerate() {
                painter.text(
                    grid.min + Vec2::new(-(index as f32 + 0.75), row as f32 + 0.5) * cell,
                    Align2::CENTER_CENTER,
                    value,
                    font.clone(),
                    color,
                );
            }
            if self.show_spans {
                let span = clue_span(clue);
                painter.text(
                    grid.min + Vec2::new(span_x, row as f32 + 0.5) * cell,
                    Align2::CENTER_CENTER,
                    format!("({span})"),
                    span_font.clone(),
                    if span == n { theme::GOLD } else { theme::MUTED },
                );
            }
        }
        for col in 0..n {
            let clue = &self.game.column_clues()[col];
            let color = if self.game.column_complete(col) {
                theme::MUTED
            } else {
                theme::CREAM
            };
            for (index, value) in clue.iter().rev().enumerate() {
                painter.text(
                    grid.min + Vec2::new(col as f32 + 0.5, -(index as f32 + 0.75)) * cell,
                    Align2::CENTER_CENTER,
                    value,
                    font.clone(),
                    color,
                );
            }
            if self.show_spans {
                let span = clue_span(clue);
                painter.text(
                    grid.min + Vec2::new(col as f32 + 0.5, span_y) * cell,
                    Align2::CENTER_CENTER,
                    format!("({span})"),
                    span_font.clone(),
                    if span == n { theme::GOLD } else { theme::MUTED },
                );
            }
        }
        for row in 0..n {
            for col in 0..n {
                let rect = Rect::from_min_size(
                    grid.min + Vec2::new(col as f32, row as f32) * cell,
                    Vec2::splat(cell),
                );
                let value = self.displayed_cell(row, col);
                let fill = if value == Cell::Filled {
                    theme::GOLD
                } else {
                    theme::CREAM
                };
                painter.rect_filled(rect, 0.0, fill);
                if value == Cell::Crossed {
                    let mark = rect.shrink(cell * 0.30);
                    let stroke = Stroke::new(1.6, Color32::from_rgb(119, 138, 123));
                    painter.line_segment([mark.left_top(), mark.right_bottom()], stroke);
                    painter.line_segment([mark.right_top(), mark.left_bottom()], stroke);
                }
                if self.selected == (row, col) && !self.game.is_won() {
                    painter.rect_stroke(
                        rect.shrink(3.0),
                        1.0,
                        Stroke::new(2.0, Color32::from_rgb(44, 122, 160)),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        for line in 0..=n {
            let strong = line % 5 == 0 || line == n;
            let stroke = Stroke::new(
                if strong { 2.5 } else { 1.0 },
                if strong {
                    theme::CHROME
                } else {
                    Color32::from_rgb(166, 179, 165)
                },
            );
            let offset = line as f32 * cell;
            painter.line_segment(
                [
                    grid.min + Vec2::new(offset, 0.0),
                    grid.min + Vec2::new(offset, grid.height()),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    grid.min + Vec2::new(0.0, offset),
                    grid.min + Vec2::new(grid.width(), offset),
                ],
                stroke,
            );
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let mut close = false;
        let mut start = None;
        let mut restart = false;
        let response = egui::Modal::new(Id::new("nonogram-dialog")).show(ctx, |ui| {
            ui.set_max_width(410.0);
            match &mut dialog {
                Dialog::Help => {
                    ui.heading("How to play Nonogram");
                    for text in [
                        "Reveal a picture using the numbers beside each row and above each column.",
                        "Each number is a run of filled squares, in order. Leave at least one empty square between runs.",
                        "For example, 2 1 means two filled squares, a gap of at least one, then one filled square. A 0 means the line is empty.",
                        "Click or tap to use the selected mark. Click again to erase it. Drag across squares to mark a whole run at once, then undo it as one step.",
                        "Right-click places an X; right-drag crosses several squares. On touch screens choose Cross.",
                        "Line spans shows each clue's minimum length: filled squares plus the gaps between runs. If it matches the board size, the whole line is determined and the span turns gold.",
                        "Arrow keys move the blue cursor. Space fills, X crosses, and Backspace / Delete erases. L toggles line spans.",
                        "Clues dim when the visible filled runs match. A dimmed clue alone does not guarantee those squares are correct.",
                        "Fill the complete picture to win. Crossing every empty square is optional.",
                        "Cmd / Ctrl + Z: Undo. Add Shift to Redo. Every puzzle has one solution.",
                        "New random generates another unique puzzle. Enter a seed to replay the same one.",
                    ] { ui.label(text); }
                    close = ui.button("Close").clicked();
                }
                Dialog::Start(source) => {
                    ui.heading("Change puzzle?");
                    ui.label("Your current marks and undo history will be reset.");
                    ui.horizontal(|ui| {
                        if ui.button("Start puzzle").clicked() { start = Some(*source); }
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Restart => {
                    ui.heading("Restart this puzzle?");
                    ui.label("Reset your marks and history, keeping the same puzzle.");
                    ui.horizontal(|ui| {
                        restart = ui.button("Restart").clicked();
                        close = ui.button("Cancel").clicked();
                    });
                }
                Dialog::Random { seed, size, error } => {
                    ui.heading("Random nonogram");
                    ui.horizontal(|ui| { for choice in Size::ALL { ui.radio_value(size, choice, choice.label()); } });
                    ui.label("Seed (leave empty for a new random puzzle)");
                    ui.add(egui::TextEdit::singleline(seed).hint_text("e.g. 42"));
                    ui.label("Every puzzle has one solution. Starting resets your current progress.");
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
            self.stroke = None;
        } else if let Some(source) = start {
            self.start(source);
        } else if !close && !response.should_close() {
            self.dialog = Some(dialog);
        }
    }

    fn status(&mut self, ui: &mut egui::Ui) {
        if !self.game.is_won() {
            let text = if self.show_spans {
                "Line spans show filled squares plus gaps. Gold spans equal the board size, so the whole line is forced. Hint: hide spans"
            } else {
                "Numbers give filled runs in order, with a gap between runs. Drag to mark · Space: fill · X: cross · Delete: erase · Hint: line spans"
            };
            ui.label(RichText::new(text).color(theme::MUTED));
            return;
        }
        let source = self.game.source();
        let mut next = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("Picture complete: {}!", self.game.name()))
                    .size(20.0)
                    .color(theme::GOLD),
            );
            ui.add_enabled_ui(self.dialog.is_none(), |ui| match source {
                Source::Puzzle(puzzle) if puzzle + 1 < PUZZLE_COUNT => {
                    if ui.button("Next puzzle").clicked() {
                        next = Some(Source::Puzzle(puzzle + 1));
                    }
                }
                Source::Random { size, .. } => {
                    if ui.button("New random puzzle").clicked() {
                        next = Some(Source::Random {
                            seed: crate::random_seed(),
                            size,
                        });
                    }
                }
                _ => {
                    ui.label(RichText::new("Try New random for more puzzles.").color(theme::MUTED));
                }
            });
        });
        if let Some(source) = next {
            self.start(source);
        }
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.shortcuts(&ctx);
        egui::Panel::top("nonogram-toolbar")
            .frame(
                egui::Frame::new()
                    .fill(theme::CHROME)
                    .inner_margin(egui::Margin::symmetric(16, 8)),
            )
            .show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("nonogram-status")
            .frame(egui::Frame::new().fill(theme::CHROME).inner_margin(12))
            .show(ui, |ui| self.status(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BOARD).inner_margin(16))
            .show(ui, |ui| {
                let (left, top) = self.clue_space();
                let min_cell = if self.game.size() >= 15 { 16.0 } else { 24.0 };
                let cell = ((ui.available_width() - 4.0) / (self.game.size() as f32 + left))
                    .min((ui.available_height() - 4.0) / (self.game.size() as f32 + top))
                    .clamp(min_cell, 48.0);
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_enabled_ui(self.dialog.is_none(), |ui| self.board(ui, cell));
                    });
                });
            });
        self.dialogs(&ctx);
    }
}

impl eframe::App for NonogramApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

#[cfg(test)]
mod tests;
