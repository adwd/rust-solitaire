use eframe::egui::{self, Color32, RichText};

const BAR: Color32 = Color32::from_rgb(10, 30, 26);
const GOLD: Color32 = Color32::from_rgb(225, 196, 126);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKind {
    Klondike,
    Sudoku,
    Game2048,
    Minesweeper,
}

impl GameKind {
    pub const ALL: [Self; 4] = [
        Self::Klondike,
        Self::Sudoku,
        Self::Game2048,
        Self::Minesweeper,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Klondike => "Klondike",
            Self::Sudoku => "Sudoku",
            Self::Game2048 => "2048",
            Self::Minesweeper => "Minesweeper",
        }
    }

    /// The URL fragment that opens this game, without the leading `#`.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Klondike => "klondike",
            Self::Sudoku => "sudoku",
            Self::Game2048 => "2048",
            Self::Minesweeper => "minesweeper",
        }
    }

    /// Accepts a slug with or without `#`; anything else is `None`.
    pub fn from_slug(value: &str) -> Option<Self> {
        let value = value.trim_start_matches('#');
        Self::ALL
            .into_iter()
            .find(|kind| kind.slug().eq_ignore_ascii_case(value))
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// Every game stays alive in the background, so switching keeps progress.
pub struct GamesApp {
    current: GameKind,
    games: [Box<dyn eframe::App>; 4],
    on_switch: Box<dyn FnMut(GameKind)>,
}

impl GamesApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        initial: GameKind,
        on_switch: Box<dyn FnMut(GameKind)>,
    ) -> Self {
        let games: [Box<dyn eframe::App>; 4] = [
            Box::new(klondike_egui::KlondikeApp::new(
                cc,
                klondike_egui::random_seed(),
                klondike_core::Rules::default(),
            )),
            Box::new(sudoku_web::SudokuApp::new(
                cc,
                sudoku_web::random_seed(),
                sudoku_core::Difficulty::default(),
            )),
            Box::new(game2048_web::Game2048App::new(
                cc,
                game2048_web::random_seed(),
            )),
            Box::new(minesweeper_web::MinesweeperApp::new(
                cc,
                minesweeper_web::random_seed(),
                minesweeper_core::Config::BEGINNER,
            )),
        ];
        Self::with_games(initial, games, on_switch)
    }

    fn with_games(
        current: GameKind,
        games: [Box<dyn eframe::App>; 4],
        on_switch: Box<dyn FnMut(GameKind)>,
    ) -> Self {
        Self {
            current,
            games,
            on_switch,
        }
    }

    pub fn current(&self) -> GameKind {
        self.current
    }

    pub fn switch(&mut self, kind: GameKind) {
        if kind != self.current {
            self.current = kind;
            (self.on_switch)(kind);
        }
    }

    /// Returns the game chosen in the switcher this frame, if any.
    fn switcher(&self, ui: &mut egui::Ui) -> Option<GameKind> {
        let mut chosen = None;
        egui::Panel::top("game-switcher")
            .frame(
                egui::Frame::new()
                    .fill(BAR)
                    .inner_margin(egui::Margin::symmetric(24, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("GAMES").size(11.0).color(GOLD).strong());
                    for kind in GameKind::ALL {
                        if ui
                            .selectable_label(kind == self.current, kind.label())
                            .clicked()
                        {
                            chosen = Some(kind);
                        }
                    }
                });
            });
        chosen
    }
}

impl eframe::App for GamesApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        if let Some(kind) = self.switcher(ui) {
            self.switch(kind);
        }
        self.games[self.current.index()].ui(ui, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn slugs_round_trip_and_ignore_the_hash_sign() {
        for kind in GameKind::ALL {
            assert_eq!(GameKind::from_slug(kind.slug()), Some(kind));
            assert_eq!(
                GameKind::from_slug(&format!("#{}", kind.slug())),
                Some(kind)
            );
        }
        assert_eq!(GameKind::from_slug("#SUDOKU"), Some(GameKind::Sudoku));
        assert_eq!(GameKind::from_slug(""), None);
        assert_eq!(GameKind::from_slug("#chess"), None);
    }

    #[test]
    fn switching_reports_only_real_changes() {
        struct Blank;
        impl eframe::App for Blank {
            fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
        }
        let seen = Rc::new(RefCell::new(Vec::new()));
        let record = Rc::clone(&seen);
        let mut app = GamesApp::with_games(
            GameKind::Klondike,
            std::array::from_fn(|_| Box::new(Blank) as Box<dyn eframe::App>),
            Box::new(move |kind| record.borrow_mut().push(kind)),
        );
        app.switch(GameKind::Klondike);
        app.switch(GameKind::Minesweeper);
        app.switch(GameKind::Sudoku);
        assert_eq!(app.current(), GameKind::Sudoku);
        assert_eq!(*seen.borrow(), [GameKind::Minesweeper, GameKind::Sudoku]);
    }
}
