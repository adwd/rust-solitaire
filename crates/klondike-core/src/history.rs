use crate::game::State;

#[derive(Debug, Default)]
pub(crate) struct History {
    past: Vec<State>,
    future: Vec<State>,
}

impl History {
    pub fn record(&mut self, before: State) {
        self.past.push(before);
        self.future.clear();
    }

    pub fn undo(&mut self, state: &mut State) -> bool {
        if let Some(previous) = self.past.pop() {
            self.future.push(std::mem::replace(state, previous));
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, state: &mut State) -> bool {
        if let Some(next) = self.future.pop() {
            self.past.push(std::mem::replace(state, next));
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
}
