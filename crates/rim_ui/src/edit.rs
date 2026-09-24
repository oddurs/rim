//! A text input's buffer, caret and selection. Indices are chars, not
//! bytes, so every operation is a whole character. Kept by the engine,
//! keyed by the node's id, because the tree that shows it is rebuilt
//! twenty times a second.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditState {
    pub text: String,
    /// Where typing goes, in chars from the start.
    pub caret: usize,
    /// The other end of the selection; equal to `caret` when nothing is
    /// selected.
    pub anchor: usize,
}

impl EditState {
    pub fn at_end(text: &str) -> EditState {
        let n = text.chars().count();
        EditState { text: text.to_string(), caret: n, anchor: n }
    }

    /// The selected range, low to high, in chars.
    pub fn selection(&self) -> (usize, usize) {
        (self.caret.min(self.anchor), self.caret.max(self.anchor))
    }

    fn byte(&self, ch: usize) -> usize {
        self.text.char_indices().nth(ch).map_or(self.text.len(), |(b, _)| b)
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// Remove the selection, if any. True when something went.
    fn cut_selection(&mut self) -> bool {
        let (lo, hi) = self.selection();
        if lo == hi {
            return false;
        }
        let (a, b) = (self.byte(lo), self.byte(hi));
        self.text.replace_range(a..b, "");
        self.caret = lo;
        self.anchor = lo;
        true
    }

    pub fn insert(&mut self, s: &str) {
        self.cut_selection();
        let at = self.byte(self.caret);
        self.text.insert_str(at, s);
        self.caret += s.chars().count();
        self.anchor = self.caret;
    }

    pub fn backspace(&mut self) -> bool {
        if self.cut_selection() {
            return true;
        }
        if self.caret == 0 {
            return false;
        }
        let (a, b) = (self.byte(self.caret - 1), self.byte(self.caret));
        self.text.replace_range(a..b, "");
        self.caret -= 1;
        self.anchor = self.caret;
        true
    }

    pub fn delete(&mut self) -> bool {
        if self.cut_selection() {
            return true;
        }
        if self.caret >= self.len() {
            return false;
        }
        let (a, b) = (self.byte(self.caret), self.byte(self.caret + 1));
        self.text.replace_range(a..b, "");
        true
    }

    /// Move the caret to `to`; with `extend` the anchor stays (a selection).
    pub fn move_to(&mut self, to: usize, extend: bool) {
        self.caret = to.min(self.len());
        if !extend {
            self.anchor = self.caret;
        }
    }

    pub fn left(&mut self, extend: bool) {
        let (lo, hi) = self.selection();
        // A plain arrow with a selection collapses to its near end.
        if !extend && lo != hi {
            self.move_to(lo, false);
        } else {
            self.move_to(self.caret.saturating_sub(1), extend);
        }
    }

    pub fn right(&mut self, extend: bool) {
        let (lo, hi) = self.selection();
        if !extend && lo != hi {
            self.move_to(hi, false);
        } else {
            self.move_to(self.caret + 1, extend);
        }
    }

    pub fn home(&mut self, extend: bool) {
        self.move_to(0, extend);
    }

    pub fn end(&mut self, extend: bool) {
        self.move_to(self.len(), extend);
    }

    /// The text before the caret and the selected text, for drawing.
    pub fn prefix(&self, to: usize) -> &str {
        &self.text[..self.byte(to)]
    }
}

#[cfg(test)]
mod tests {
    use super::EditState;

    #[test]
    fn edits_work_in_chars() {
        let mut e = EditState::at_end("héllo");
        assert_eq!(e.caret, 5);
        e.left(false);
        e.left(false);
        e.insert("XY");
        assert_eq!(e.text, "hélXYlo");
        e.backspace();
        assert_eq!(e.text, "hélXlo");
        e.home(false);
        e.right(true);
        e.right(true);
        assert_eq!(e.selection(), (0, 2));
        e.insert("J");
        assert_eq!(e.text, "JlXlo");
        e.end(false);
        assert!(!e.delete());
        e.left(false);
        assert!(e.delete());
        assert_eq!(e.text, "JlXl");
        assert_eq!(e.prefix(2), "Jl");
    }
}
