#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameKind {
    Klondike,
    Sudoku,
    Game2048,
    Minesweeper,
    Sokoban,
    Nonogram,
}

impl GameKind {
    pub const ALL: [Self; 6] = [
        Self::Klondike,
        Self::Sudoku,
        Self::Game2048,
        Self::Minesweeper,
        Self::Sokoban,
        Self::Nonogram,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Klondike => "Klondike",
            Self::Sudoku => "Sudoku",
            Self::Game2048 => "2048",
            Self::Minesweeper => "Minesweeper",
            Self::Sokoban => "Sokoban",
            Self::Nonogram => "Nonogram",
        }
    }

    /// The URL fragment that opens this game, without the leading `#`.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Klondike => "klondike",
            Self::Sudoku => "sudoku",
            Self::Game2048 => "2048",
            Self::Minesweeper => "minesweeper",
            Self::Sokoban => "sokoban",
            Self::Nonogram => "nonogram",
        }
    }

    /// Accepts a slug with or without `#`; anything else is `None`.
    pub fn from_slug(value: &str) -> Option<Self> {
        let value = value.trim_start_matches('#');
        Self::ALL
            .into_iter()
            .find(|kind| kind.slug().eq_ignore_ascii_case(value))
    }

    /// Construct only a game included in this WASM module's Cargo features.
    pub fn create(self, cc: &eframe::CreationContext<'_>) -> Option<Box<dyn eframe::App>> {
        let _ = cc;
        match self {
            #[cfg(feature = "klondike")]
            Self::Klondike => Some(Box::new(klondike_egui::KlondikeApp::new(
                cc,
                klondike_egui::random_seed(),
                klondike_core::Rules::default(),
            ))),
            #[cfg(feature = "sudoku")]
            Self::Sudoku => Some(Box::new(sudoku_web::SudokuApp::new(
                cc,
                sudoku_web::random_seed(),
                sudoku_core::Difficulty::default(),
            ))),
            #[cfg(feature = "2048")]
            Self::Game2048 => Some(Box::new(game2048_web::Game2048App::new(
                cc,
                game2048_web::random_seed(),
            ))),
            #[cfg(feature = "minesweeper")]
            Self::Minesweeper => Some(Box::new(minesweeper_web::MinesweeperApp::new(
                cc,
                minesweeper_web::random_seed(),
                minesweeper_core::Config::BEGINNER,
            ))),
            #[cfg(feature = "sokoban")]
            Self::Sokoban => Some(Box::new(sokoban_web::SokobanApp::new(cc))),
            #[cfg(feature = "nonogram")]
            Self::Nonogram => Some(Box::new(nonogram_web::NonogramApp::new(cc))),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
