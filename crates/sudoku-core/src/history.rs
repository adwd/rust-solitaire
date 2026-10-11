#[derive(Debug)]
pub(crate) struct History<S> {
    past: Vec<S>,
    future: Vec<S>,
}

impl<S> Default for History<S> {
    fn default() -> Self {
        Self {
            past: Vec::new(),
            future: Vec::new(),
        }
    }
}

impl<S> History<S> {
    pub fn record(&mut self, before: S) {
        self.past.push(before);
        self.future.clear();
    }

    pub fn undo(&mut self, state: &mut S) -> bool {
        if let Some(previous) = self.past.pop() {
            self.future.push(std::mem::replace(state, previous));
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, state: &mut S) -> bool {
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
