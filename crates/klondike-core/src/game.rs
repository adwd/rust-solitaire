use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

use crate::{Action, Card, MoveError, Rank, Source, Suit, Target, history::History};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DrawMode {
    #[default]
    One,
    Three,
}

impl DrawMode {
    pub const fn count(self) -> usize {
        match self {
            Self::One => 1,
            Self::Three => 3,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Rules {
    pub draw: DrawMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Playing,
    Won,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableauCard {
    pub card: Card,
    pub face_up: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct State {
    // All piles have their exposed top at the end of the vector.
    pub stock: Vec<Card>,
    pub waste: Vec<Card>,
    pub tableau: [Vec<TableauCard>; 7],
    pub foundations: [Vec<Card>; 4],
    pub moves: u64,
}

impl State {
    pub fn status(&self) -> Status {
        if self.foundations.iter().map(Vec::len).sum::<usize>() == 52 {
            Status::Won
        } else {
            Status::Playing
        }
    }
}

/// Borrowed, read-only access: callers cannot mutate a game's piles.
#[derive(Debug, Clone, Copy)]
pub struct GameView<'a> {
    pub stock: &'a [Card],
    pub waste: &'a [Card],
    pub tableau: &'a [Vec<TableauCard>; 7],
    pub foundations: &'a [Vec<Card>; 4],
    pub moves: u64,
    pub seed: u64,
    pub rules: Rules,
    pub status: Status,
}

#[derive(Debug)]
pub struct Game {
    pub(crate) state: State,
    history: History,
    seed: u64,
    rules: Rules,
}

impl Game {
    /// Deal version 1: C/D/H/S, A..K, ChaCha8 and u32 Fisher-Yates, then pop.
    pub fn new(seed: u64, rules: Rules) -> Self {
        let mut deck: Vec<_> = Suit::ALL
            .into_iter()
            .flat_map(|suit| Rank::ALL.into_iter().map(move |rank| Card::new(suit, rank)))
            .collect();
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        for upper in (1..deck.len()).rev() {
            let index = rng.random_range(0..=upper as u32) as usize;
            deck.swap(upper, index);
        }
        let tableau = std::array::from_fn(|column| {
            (0..=column)
                .map(|index| TableauCard {
                    card: deck.pop().expect("52 cards cover the 28-card deal"),
                    face_up: index == column,
                })
                .collect()
        });
        Self {
            state: State {
                stock: deck,
                waste: Vec::new(),
                tableau,
                foundations: std::array::from_fn(|_| Vec::new()),
                moves: 0,
            },
            history: History::default(),
            seed,
            rules,
        }
    }

    pub fn view(&self) -> GameView<'_> {
        GameView {
            stock: &self.state.stock,
            waste: &self.state.waste,
            tableau: &self.state.tableau,
            foundations: &self.state.foundations,
            moves: self.state.moves,
            seed: self.seed,
            rules: self.rules,
            status: self.status(),
        }
    }

    pub fn status(&self) -> Status {
        self.state.status()
    }
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
    pub fn undo(&mut self) -> bool {
        self.history.undo(&mut self.state)
    }
    pub fn redo(&mut self) -> bool {
        self.history.redo(&mut self.state)
    }

    pub fn validate(&self, action: Action) -> Result<(), MoveError> {
        self.state.validate(action)
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        self.state.legal_actions()
    }

    /// Prove a finish using only foundations and ordinary stock operations.
    /// Hidden tableau cards defer automatic play. A failed proof changes nothing.
    /// This is deliberately not a general-purpose solver or a best-move hint.
    pub fn auto_finish_plan(&self) -> Option<Vec<Action>> {
        if self.status() == Status::Won
            || self
                .state
                .tableau
                .iter()
                .flatten()
                .any(|card| !card.face_up)
        {
            return None;
        }
        let mut simulation = Self {
            state: self.state.clone(),
            history: History::default(),
            seed: self.seed,
            rules: self.rules,
        };
        let mut plan = Vec::new();
        let mut recycles_without_progress = 0;
        while simulation.status() != Status::Won {
            let mut sources = vec![Source::Waste];
            sources.extend(simulation.state.tableau.iter().enumerate().filter_map(
                |(column, pile)| {
                    pile.len()
                        .checked_sub(1)
                        .map(|index| Source::Tableau { column, index })
                },
            ));
            let transfer = sources.into_iter().find_map(|from| {
                (0..4)
                    .map(|pile| Action::Move {
                        from,
                        to: Target::Foundation(pile),
                    })
                    .find(|action| simulation.validate(*action).is_ok())
            });
            let action = if let Some(transfer) = transfer {
                recycles_without_progress = 0;
                transfer
            } else if !simulation.state.stock.is_empty() {
                Action::Draw
            } else if !simulation.state.waste.is_empty() {
                recycles_without_progress += 1;
                // The first recycle may follow a partial pass. A second pass
                // without removing a card repeats the same stock/waste order.
                if recycles_without_progress == 2 {
                    return None;
                }
                Action::Recycle
            } else {
                return None;
            };
            simulation.apply(action).ok()?;
            simulation.history = History::default();
            plan.push(action);
        }
        Some(plan)
    }

    /// Validation happens before any mutation, including history changes.
    pub fn apply(&mut self, action: Action) -> Result<Status, MoveError> {
        self.validate(action)?;
        self.history.record(self.state.clone());
        match action {
            Action::Draw => {
                for _ in 0..self.rules.draw.count().min(self.state.stock.len()) {
                    self.state
                        .waste
                        .push(self.state.stock.pop().expect("validated stock"));
                }
            }
            Action::Recycle => {
                self.state.stock = self.state.waste.drain(..).rev().collect();
            }
            Action::Move { from, to } => {
                let cards = match from {
                    Source::Waste => vec![self.state.waste.pop().expect("validated waste")],
                    Source::Foundation(pile) => vec![
                        self.state.foundations[pile]
                            .pop()
                            .expect("validated foundation"),
                    ],
                    Source::Tableau { column, index } => {
                        let pile = &mut self.state.tableau[column];
                        let cards = pile.drain(index..).map(|placed| placed.card).collect();
                        if let Some(exposed) = pile.last_mut() {
                            exposed.face_up = true;
                        }
                        cards
                    }
                };
                match to {
                    Target::Tableau(column) => {
                        self.state.tableau[column].extend(cards.into_iter().map(|card| {
                            TableauCard {
                                card,
                                face_up: true,
                            }
                        }))
                    }
                    Target::Foundation(pile) => self.state.foundations[pile].extend(cards),
                }
            }
        }
        self.state.moves += 1;
        Ok(self.status())
    }

    #[cfg(test)]
    pub(crate) fn from_state(state: State, rules: Rules) -> Self {
        Self {
            state,
            history: History::default(),
            seed: 0,
            rules,
        }
    }
}
