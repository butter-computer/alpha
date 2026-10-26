//! Text buffer with undo/redo. Offsets are UTF-8 byte offsets.

use std::ops::Range;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
struct Edit {
    at: usize,
    removed: String,
    inserted: String,
    sel_before: Range<usize>,
    sel_after: Range<usize>,
    time: Instant,
}

#[derive(Default)]
pub struct Buffer {
    text: String,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    revision: u64,
}

const COALESCE: Duration = Duration::from_millis(900);

impl Buffer {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), ..Default::default() }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn len(&self) -> usize {
        self.text.len()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Increments on every change (including undo/redo).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Replaces the whole text and clears history.
    pub fn reset(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.undo.clear();
        self.redo.clear();
        self.revision += 1;
    }

    /// Replaces `range` with `new`. Returns the new cursor offset.
    pub fn replace(&mut self, range: Range<usize>, new: &str, sel_before: Range<usize>) -> usize {
        let removed = self.text[range.clone()].to_string();
        self.text.replace_range(range.clone(), new);
        self.revision += 1;
        self.redo.clear();
        let cursor = range.start + new.len();
        let now = Instant::now();
        if let Some(last) = self.undo.last_mut() {
            let typing = removed.is_empty()
                && !new.is_empty()
                && !new.contains('\n')
                && last.removed.is_empty()
                && !last.inserted.contains('\n')
                && last.at + last.inserted.len() == range.start
                && now.duration_since(last.time) < COALESCE;
            if typing {
                last.inserted.push_str(new);
                last.sel_after = cursor..cursor;
                last.time = now;
                return cursor;
            }
        }
        self.undo.push(Edit {
            at: range.start,
            removed,
            inserted: new.to_string(),
            sel_before,
            sel_after: cursor..cursor,
            time: now,
        });
        cursor
    }

    /// Undoes the last edit, returning the selection to restore.
    pub fn undo(&mut self) -> Option<Range<usize>> {
        let e = self.undo.pop()?;
        self.text.replace_range(e.at..e.at + e.inserted.len(), &e.removed);
        self.revision += 1;
        let sel = e.sel_before.clone();
        self.redo.push(e);
        Some(sel)
    }

    pub fn redo(&mut self) -> Option<Range<usize>> {
        let mut e = self.redo.pop()?;
        self.text.replace_range(e.at..e.at + e.removed.len(), &e.inserted);
        self.revision += 1;
        let sel = e.sel_after.clone();
        e.time = Instant::now() - COALESCE;
        self.undo.push(e);
        Some(sel)
    }

    // ---- navigation helpers -------------------------------------------------

    /// The run of same-class characters (word or punctuation) around `offset`.
    pub fn word_range(&self, offset: usize) -> Range<usize> {
        let before = self.text[..offset].chars().next_back();
        let after = self.text[offset..].chars().next();
        let class = match (after, before) {
            (Some(c), _) if !c.is_whitespace() => word_class(c),
            (_, Some(c)) if !c.is_whitespace() => word_class(c),
            _ => return offset..offset,
        };
        let start = self.text[..offset]
            .char_indices()
            .rev()
            .take_while(|(_, c)| !c.is_whitespace() && word_class(*c) == class)
            .last()
            .map(|(i, _)| i)
            .unwrap_or(offset);
        let end = offset
            + self.text[offset..]
                .char_indices()
                .take_while(|(_, c)| !c.is_whitespace() && word_class(*c) == class)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
        start..end
    }

    /// Start of the logical line containing `offset`.
    pub fn line_start(&self, offset: usize) -> usize {
        self.text[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0)
    }

    /// End of the logical line containing `offset` (before the newline).
    pub fn line_end(&self, offset: usize) -> usize {
        self.text[offset..].find('\n').map(|i| offset + i).unwrap_or(self.text.len())
    }
}

pub fn prev_grapheme_in(text: &str, offset: usize) -> usize {
    use unicode_segmentation::UnicodeSegmentation;
    text[..offset].grapheme_indices(true).next_back().map(|(i, _)| i).unwrap_or(0)
}

pub fn next_grapheme_in(text: &str, offset: usize) -> usize {
    use unicode_segmentation::UnicodeSegmentation;
    text[offset..].graphemes(true).next().map(|g| offset + g.len()).unwrap_or(text.len())
}

pub fn prev_word_in(text: &str, offset: usize) -> usize {
    let mut chars = text[..offset].char_indices().rev().peekable();
    while chars.peek().is_some_and(|(_, c)| c.is_whitespace()) {
        chars.next();
    }
    let Some(&(mut pos, first)) = chars.peek() else { return 0 };
    let class = word_class(first);
    for (i, c) in chars {
        if word_class(c) != class {
            return pos;
        }
        pos = i;
    }
    pos
}

pub fn next_word_in(text: &str, offset: usize) -> usize {
    let mut it = text[offset..].char_indices().peekable();
    while it.peek().is_some_and(|(_, c)| c.is_whitespace()) {
        it.next();
    }
    let Some(&(_, first)) = it.peek() else { return text.len() };
    let class = word_class(first);
    for (i, c) in it {
        if word_class(c) != class {
            return offset + i;
        }
    }
    text.len()
}

fn word_class(c: char) -> u8 {
    if c.is_alphanumeric() || c == '_' { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_and_undo_redo() {
        let mut b = Buffer::new("hello");
        let c = b.replace(5..5, " world", 5..5);
        assert_eq!(b.text(), "hello world");
        assert_eq!(c, 11);
        assert_eq!(b.undo(), Some(5..5));
        assert_eq!(b.text(), "hello");
        assert_eq!(b.redo(), Some(11..11));
        assert_eq!(b.text(), "hello world");
        assert_eq!(b.undo(), Some(5..5));
        assert_eq!(b.undo(), None);
    }

    #[test]
    fn typing_coalesces() {
        let mut b = Buffer::new("");
        for (i, ch) in "abc".chars().enumerate() {
            b.replace(i..i, &ch.to_string(), i..i);
        }
        assert_eq!(b.text(), "abc");
        assert_eq!(b.undo(), Some(0..0));
        assert_eq!(b.text(), "");
    }

    #[test]
    fn deletion_undo_restores() {
        let mut b = Buffer::new("abcdef");
        b.replace(1..4, "", 1..4);
        assert_eq!(b.text(), "aef");
        assert_eq!(b.undo(), Some(1..4));
        assert_eq!(b.text(), "abcdef");
    }

    #[test]
    fn revision_changes() {
        let mut b = Buffer::new("a");
        let r = b.revision();
        b.replace(0..1, "b", 0..1);
        assert_ne!(r, b.revision());
    }

    #[test]
    fn graphemes_and_words() {
        let b = Buffer::new("héllo wörld  foo");
        assert_eq!(next_grapheme_in(b.text(), 0), 1);
        assert_eq!(next_grapheme_in(b.text(), 1), 3);
        assert_eq!(prev_grapheme_in(b.text(), 3), 1);
        assert_eq!(next_word_in(b.text(), 0), 6);
        assert_eq!(next_word_in(b.text(), 6), 13);
        assert_eq!(next_word_in(b.text(), b.len()), b.len());
        assert_eq!(prev_word_in(b.text(), b.len()), 15);
        assert_eq!(prev_word_in(b.text(), 15), 7);
        assert_eq!(prev_word_in(b.text(), 7), 0);
        assert_eq!(prev_word_in(b.text(), 0), 0);
    }

    #[test]
    fn word_ranges() {
        let b = Buffer::new("foo bar, baz");
        assert_eq!(b.word_range(1), 0..3);
        assert_eq!(b.word_range(3), 0..3);
        assert_eq!(b.word_range(4), 4..7);
        assert_eq!(b.word_range(7), 7..8);
        assert_eq!(b.word_range(8), 7..8);
    }

    #[test]
    fn line_bounds() {
        let b = Buffer::new("ab\ncd\n");
        assert_eq!(b.line_start(4), 3);
        assert_eq!(b.line_end(3), 5);
        assert_eq!(b.line_start(6), 6);
        assert_eq!(b.line_end(0), 2);
    }
}
