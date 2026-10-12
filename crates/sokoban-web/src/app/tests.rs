use super::*;
use eframe::egui::{Event, PointerButton, Pos2};

struct Harness {
    ctx: egui::Context,
    app: SokobanApp,
}

impl Harness {
    fn new() -> Self {
        let mut h = Self {
            ctx: egui::Context::default(),
            app: SokobanApp::with_game(Game::default()),
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

    fn click(&mut self, row: usize, col: usize) {
        let rect = self.app.board_rect.unwrap();
        let cell = rect.width() / self.app.game.width() as f32;
        let pos = rect.min + Vec2::new(col as f32 + 0.5, row as f32 + 0.5) * cell;
        self.frame(vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
    }
}

#[test]
fn keyboard_pushes_win_and_shortcuts_restore_progress() {
    let mut h = Harness::new();
    h.key(Key::D, Modifiers::NONE);
    assert_eq!(h.app.game.pushes(), 1);
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert!(h.app.game.is_won());
    h.key(Key::Z, Modifiers::COMMAND);
    assert!(!h.app.game.is_won());
    h.key(Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert!(h.app.game.is_won());
}

#[test]
fn click_moves_only_to_an_adjacent_square() {
    let mut h = Harness::new();
    h.click(2, 5);
    assert_eq!(h.app.game.moves(), 0);
    h.click(2, 3);
    assert_eq!(h.app.game.pushes(), 1);
    h.click(2, 4);
    assert!(h.app.game.is_won());
}

#[test]
fn dialog_blocks_moves_and_level_change_resets_history() {
    let mut h = Harness::new();
    h.key(Key::D, Modifiers::NONE);
    h.app.request_start(Source::Level(1));
    assert!(h.app.dialog.is_some());
    h.key(Key::D, Modifiers::NONE);
    h.click(2, 4);
    assert_eq!(h.app.game.moves(), 1);
    h.app.start(Source::Level(1));
    assert_eq!(h.app.game.level(), Some(1));
    assert_eq!(h.app.game.moves(), 0);
    assert!(!h.app.game.can_undo());
}
