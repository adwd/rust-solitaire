use super::*;
use eframe::egui::{Event, PointerButton, Pos2};

struct Harness {
    ctx: egui::Context,
    app: NonogramApp,
}

impl Harness {
    fn new() -> Self {
        let mut h = Self {
            ctx: egui::Context::default(),
            app: NonogramApp::with_game(Game::default()),
        };
        h.frame(vec![]);
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        let _ = self.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(900.0, 800.0))),
                events,
                ..Default::default()
            },
            |ui| self.app.show(ui),
        );
    }

    fn key(&mut self, key: Key, modifiers: Modifiers) {
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }]);
        self.frame(vec![Event::Key {
            key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        }]);
    }

    fn click(&mut self, row: usize, col: usize, button: PointerButton) {
        let rect = self.app.board_rect.unwrap();
        let cell = rect.width() / self.app.game.size() as f32;
        let pos = rect.min + Vec2::new(col as f32 + 0.5, row as f32 + 0.5) * cell;
        self.frame(vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        self.frame(vec![Event::PointerButton {
            pos,
            button,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
    }
}

#[test]
fn primary_secondary_and_touch_mode_marks_can_be_erased() {
    let mut h = Harness::new();
    h.click(0, 2, PointerButton::Primary);
    assert_eq!(h.app.game.cell(0, 2), Some(Cell::Filled));
    h.click(0, 2, PointerButton::Primary);
    assert_eq!(h.app.game.cell(0, 2), Some(Cell::Unknown));
    h.click(0, 0, PointerButton::Secondary);
    assert_eq!(h.app.game.cell(0, 0), Some(Cell::Crossed));
    h.app.mode = Cell::Crossed;
    h.click(0, 1, PointerButton::Primary);
    assert_eq!(h.app.game.cell(0, 1), Some(Cell::Crossed));
    h.app.mode = Cell::Unknown;
    h.click(0, 1, PointerButton::Primary);
    assert_eq!(h.app.game.cell(0, 1), Some(Cell::Unknown));
}

#[test]
fn keyboard_selection_marks_and_history() {
    let mut h = Harness::new();
    h.key(Key::ArrowRight, Modifiers::NONE);
    h.key(Key::ArrowDown, Modifiers::NONE);
    assert_eq!(h.app.selected, (1, 1));
    h.key(Key::Space, Modifiers::NONE);
    assert_eq!(h.app.game.cell(1, 1), Some(Cell::Filled));
    h.key(Key::X, Modifiers::NONE);
    assert_eq!(h.app.game.cell(1, 1), Some(Cell::Crossed));
    h.key(Key::Z, Modifiers::COMMAND);
    assert_eq!(h.app.game.cell(1, 1), Some(Cell::Filled));
    h.key(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(h.app.game.cell(1, 1), Some(Cell::Crossed));
    h.key(Key::Delete, Modifiers::NONE);
    assert_eq!(h.app.game.cell(1, 1), Some(Cell::Unknown));
}

#[test]
fn completing_the_picture_and_undoing_work_through_pointer_input() {
    let mut h = Harness::new();
    for row in 0..5 {
        for col in 0..5 {
            if row == 2 || col == 2 {
                h.click(row, col, PointerButton::Primary);
            }
        }
    }
    assert!(h.app.game.is_won());
    h.key(Key::Z, Modifiers::COMMAND);
    assert!(!h.app.game.is_won());
    h.key(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(h.app.game.is_won());
}

#[test]
fn modal_blocks_marks_and_switching_resets_selection_and_history() {
    let mut h = Harness::new();
    h.click(0, 2, PointerButton::Primary);
    h.app.request_start(Source::Puzzle(5));
    assert!(h.app.dialog.is_some());
    h.key(Key::Space, Modifiers::NONE);
    h.click(1, 2, PointerButton::Primary);
    assert_eq!(h.app.game.moves(), 1);
    h.app.start(Source::Puzzle(5));
    assert_eq!(h.app.game.size(), 10);
    assert_eq!(h.app.selected, (0, 0));
    assert!(!h.app.game.can_undo());
}
