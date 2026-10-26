//! Typing shortcuts that rewrite what the user just typed.
//!
//! Almost every markdown shortcut needs no rewriting: the buffer is already
//! markdown and the parser recognises `# `, `- `, `1. `, `> ` and a fence as soon
//! as they read that way. The one rewrite is the checklist spelling: `[] ` and
//! `[ ] ` at the start of a line become `- [ ] `.

use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub struct Edit {
    pub range: Range<usize>,
    pub text: String,
    pub caret: usize,
}

fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map(|i| i + 1).unwrap_or(0)
}

/// Called after `typed` (a single character) was inserted and the caret sits at `caret`.
pub fn after_insert(text: &str, caret: usize, typed: &str, in_code: bool) -> Option<Edit> {
    if in_code || typed != " " {
        return None;
    }
    let ls = line_start(text, caret);
    let before = &text[ls..caret];
    let indent_len = before.len() - before.trim_start_matches(' ').len();
    let rest = &before[indent_len..];
    if rest == "[] " || rest == "[ ] " {
        let t = format!("{}- [ ] ", &before[..indent_len]);
        return Some(Edit { range: ls..caret, caret: ls + t.len(), text: t });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checklist_shortcut_normalizes() {
        let e = after_insert("[] ", 3, " ", false).unwrap();
        assert_eq!(e, Edit { range: 0..3, text: "- [ ] ".into(), caret: 6 });
        let e = after_insert("a\n  [ ] ", 8, " ", false).unwrap();
        assert_eq!(e.range, 2..8);
        assert_eq!(e.text, "  - [ ] ");
        assert!(after_insert("x [] ", 5, " ", false).is_none());
        assert!(after_insert("[] ", 3, " ", true).is_none());
    }

    #[test]
    fn other_typing_is_untouched() {
        assert!(after_insert("# ", 2, " ", false).is_none());
        assert!(after_insert("- ", 2, " ", false).is_none());
        assert!(after_insert("```", 3, "`", false).is_none());
        assert!(after_insert("a", 1, "a", false).is_none());
    }
}
