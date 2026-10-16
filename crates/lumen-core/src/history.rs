/// Generic undo/redo history with bounded capacity.

#[derive(Clone, Debug)]
pub struct History<T: Clone> {
    undo_stack: Vec<T>,
    redo_stack: Vec<T>,
    capacity: usize,
}

impl<T: Clone> History<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            capacity: capacity.max(1),
        }
    }

    /// Record a snapshot before a change. Pushes `before` onto the undo stack
    /// and clears the redo stack.
    pub fn record(&mut self, before: T) {
        if self.undo_stack.len() >= self.capacity {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(before);
        self.redo_stack.clear();
    }

    /// Undo: returns the previous state if available.
    /// `current` is pushed onto the redo stack.
    pub fn undo(&mut self, current: T) -> Option<T> {
        let previous = self.undo_stack.pop()?;
        self.redo_stack.push(current);
        Some(previous)
    }

    /// Redo: returns the next state if available.
    /// `current` is pushed onto the undo stack.
    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(current);
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

impl<T: Clone> Default for History<T> {
    fn default() -> Self {
        Self::new(200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_undo_redo() {
        let mut h = History::new(10);
        h.record(1);
        h.record(2);
        assert!(h.can_undo());
        assert!(!h.can_redo());

        // current is 3, undo should give 2
        let prev = h.undo(3).unwrap();
        assert_eq!(prev, 2);
        assert!(h.can_redo());

        let prev = h.undo(2).unwrap();
        assert_eq!(prev, 1);

        assert!(h.undo(1).is_none());

        // redo
        let next = h.redo(1).unwrap();
        assert_eq!(next, 2);
        let next = h.redo(2).unwrap();
        assert_eq!(next, 3);
        assert!(h.redo(3).is_none());
    }

    #[test]
    fn record_clears_redo() {
        let mut h = History::new(10);
        h.record(1);
        h.record(2);
        let _ = h.undo(3); // redo now has 3
        assert!(h.can_redo());

        h.record(10); // branch: redo cleared
        assert!(!h.can_redo());
    }

    #[test]
    fn capacity_drops_oldest() {
        let mut h = History::new(3);
        h.record(1);
        h.record(2);
        h.record(3);
        // At capacity
        h.record(4); // oldest (1) dropped
        assert_eq!(h.undo_stack.len(), 3);

        let v = h.undo(5).unwrap();
        assert_eq!(v, 4);
        let v = h.undo(4).unwrap();
        assert_eq!(v, 3);
        let v = h.undo(3).unwrap();
        assert_eq!(v, 2);
        assert!(h.undo(2).is_none()); // 1 was dropped
    }

    #[test]
    fn clear_empties_both_stacks() {
        let mut h = History::new(10);
        h.record(1);
        h.record(2);
        let _ = h.undo(3);
        assert!(h.can_undo());
        assert!(h.can_redo());
        h.clear();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
    }

    #[test]
    fn default_capacity_is_200() {
        let h: History<i32> = History::default();
        assert_eq!(h.capacity, 200);
    }

    #[test]
    fn empty_history_returns_none() {
        let mut h: History<i32> = History::new(5);
        assert!(!h.can_undo());
        assert!(!h.can_redo());
        assert!(h.undo(0).is_none());
        assert!(h.redo(0).is_none());
    }
}
