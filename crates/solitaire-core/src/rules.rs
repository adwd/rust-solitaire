use crate::{Action, Card, MoveError, Rank, Source, Status, Suit, Target, game::State};

impl State {
    pub fn source_cards(&self, from: Source) -> Result<Vec<Card>, MoveError> {
        match from {
            Source::Waste => self
                .waste
                .last()
                .copied()
                .map(|card| vec![card])
                .ok_or(MoveError::EmptySource),
            Source::Foundation(index) => self
                .foundations
                .get(index)
                .ok_or(MoveError::InvalidIndex)?
                .last()
                .copied()
                .map(|card| vec![card])
                .ok_or(MoveError::EmptySource),
            Source::Tableau { column, index } => {
                let pile = self.tableau.get(column).ok_or(MoveError::InvalidIndex)?;
                let cards = pile
                    .get(index..)
                    .filter(|cards| !cards.is_empty())
                    .ok_or(MoveError::InvalidIndex)?;
                if cards.iter().any(|placed| !placed.face_up) {
                    return Err(MoveError::FaceDown);
                }
                if !cards.windows(2).all(|pair| {
                    pair[0].card.color() != pair[1].card.color()
                        && pair[0].card.rank.value() == pair[1].card.rank.value() + 1
                }) {
                    return Err(MoveError::InvalidSequence);
                }
                Ok(cards.iter().map(|placed| placed.card).collect())
            }
        }
    }

    pub fn validate(&self, action: Action) -> Result<(), MoveError> {
        if self.status() == Status::Won {
            return Err(MoveError::GameWon);
        }
        match action {
            Action::Draw => {
                if self.stock.is_empty() {
                    Err(MoveError::EmptyStock)
                } else {
                    Ok(())
                }
            }
            Action::Recycle => {
                if !self.stock.is_empty() {
                    Err(MoveError::StockNotEmpty)
                } else if self.waste.is_empty() {
                    Err(MoveError::EmptyWaste)
                } else {
                    Ok(())
                }
            }
            Action::Move { from, to } => {
                if matches!((from, to), (Source::Tableau { column, .. }, Target::Tableau(dest)) if column == dest)
                    || matches!((from, to), (Source::Foundation(pile), Target::Foundation(dest)) if pile == dest)
                {
                    return Err(MoveError::SamePile);
                }
                let cards = self.source_cards(from)?;
                let bottom = cards[0];
                match to {
                    Target::Tableau(column) => {
                        let pile = self.tableau.get(column).ok_or(MoveError::InvalidIndex)?;
                        if let Some(top) = pile.last() {
                            if !top.face_up {
                                return Err(MoveError::FaceDown);
                            }
                            if top.card.color() == bottom.color()
                                || top.card.rank.value() != bottom.rank.value() + 1
                            {
                                return Err(MoveError::WrongColorOrRank);
                            }
                        } else if bottom.rank != Rank::King {
                            return Err(MoveError::RequiresKing);
                        }
                        Ok(())
                    }
                    Target::Foundation(index) => {
                        let pile = self.foundations.get(index).ok_or(MoveError::InvalidIndex)?;
                        if cards.len() != 1 {
                            return Err(MoveError::SingleCardRequired);
                        }
                        let next = pile.last().map_or(1, |card| card.rank.value() + 1);
                        if bottom.suit != Suit::ALL[index] || bottom.rank.value() != next {
                            return Err(MoveError::WrongFoundation);
                        }
                        Ok(())
                    }
                }
            }
        }
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();
        let mut sources = vec![Source::Waste];
        sources.extend((0..4).map(Source::Foundation));
        for (column, pile) in self.tableau.iter().enumerate() {
            sources.extend((0..pile.len()).map(|index| Source::Tableau { column, index }));
        }
        for from in sources {
            // Put foundation moves first, which also gives hints a useful default.
            for to in (0..4)
                .map(Target::Foundation)
                .chain((0..7).map(Target::Tableau))
            {
                let action = Action::Move { from, to };
                if self.validate(action).is_ok() {
                    actions.push(action);
                }
            }
        }
        for action in [Action::Draw, Action::Recycle] {
            if self.validate(action).is_ok() {
                actions.push(action);
            }
        }
        actions
    }
}
