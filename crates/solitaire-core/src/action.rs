use std::{error::Error, fmt};

/// Tableau indices run from the bottom to the exposed top of a pile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    Tableau { column: usize, index: usize },
    Waste,
    Foundation(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Tableau(usize),
    Foundation(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Draw,
    Recycle,
    Move { from: Source, to: Target },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveError {
    GameWon,
    EmptyStock,
    StockNotEmpty,
    EmptyWaste,
    EmptySource,
    InvalidIndex,
    FaceDown,
    InvalidSequence,
    SamePile,
    RequiresKing,
    WrongColorOrRank,
    WrongFoundation,
    SingleCardRequired,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::GameWon => "the game has already been won",
            Self::EmptyStock => "the stock is empty",
            Self::StockNotEmpty => "finish the stock before recycling",
            Self::EmptyWaste => "the waste is empty",
            Self::EmptySource => "the source pile is empty",
            Self::InvalidIndex => "the pile or card index is out of range",
            Self::FaceDown => "face-down cards cannot be moved",
            Self::InvalidSequence => "the selected cards are not a descending alternating sequence",
            Self::SamePile => "source and target must be different piles",
            Self::RequiresKing => "an empty tableau column requires a king",
            Self::WrongColorOrRank => "tableau cards must alternate colors and descend by one rank",
            Self::WrongFoundation => "foundations must match suit and ascend from ace",
            Self::SingleCardRequired => "only one card can move to a foundation",
        })
    }
}

impl Error for MoveError {}
