//! Markdown shortcuts that rewrite what the user just typed.
//!
//! Most shortcuts need no rewriting because the buffer is already markdown
//! (`# `, `- `, `1. `, `> ` are recognised by the scanner as soon as they are
//! typed). The few that do are handled here.

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

/// Whether the line above `ls` has content (a divider typed right under it
/// would otherwise turn it into a setext heading).
pub fn prev_line_nonblank(text: &str, ls: usize) -> bool {
    ls > 0 && text[..ls - 1].rsplit('\n').next().is_some_and(|l| !l.trim().is_empty())
}

/// Called after `typed` (a single character) was inserted and the caret sits at `caret`.
pub fn after_insert(text: &str, caret: usize, typed: &str, in_code: bool) -> Option<Edit> {
    if in_code || typed.chars().count() != 1 {
        return None;
    }
    let ls = line_start(text, caret);
    let before = &text[ls..caret];
    let at_end = text[caret..].is_empty() || text[caret..].starts_with('\n');
    match typed {
        " " => {
            let indent_len = before.len() - before.trim_start_matches(' ').len();
            let rest = &before[indent_len..];
            if rest == "[] " || rest == "[ ] " {
                let t = format!("{}- [ ] ", &before[..indent_len]);
                return Some(Edit { range: ls..caret, caret: ls + t.len(), text: t });
            }
            None
        }
        "`" if before == "```" && at_end => {
            Some(Edit { range: ls..caret, text: "```\n\n```".into(), caret: ls + 4 })
        }
        "-" if before == "---" && at_end && prev_line_nonblank(text, ls) => {
            Some(Edit { range: ls..caret, text: "\n---".into(), caret: ls + 4 })
        }
        _ => None,
    }
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
    }

    #[test]
    fn code_fence_gets_closing_fence() {
        let e = after_insert("```", 3, "`", false).unwrap();
        assert_eq!(e.text, "```\n\n```");
        assert_eq!(e.caret, 4);
        assert!(after_insert("```", 3, "`", true).is_none());
        // not when text follows on the line
        assert!(after_insert("```x", 3, "`", false).is_none());
    }

    #[test]
    fn divider_gets_blank_line_above_when_needed() {
        let e = after_insert("text\n---", 8, "-", false).unwrap();
        assert_eq!(e.range, 5..8);
        assert_eq!(e.text, "\n---");
        assert!(after_insert("text\n\n---", 9, "-", false).is_none());
        assert!(after_insert("---", 3, "-", false).is_none());
    }

    #[test]
    fn other_typing_is_untouched() {
        assert!(after_insert("# ", 2, " ", false).is_none());
        assert!(after_insert("- ", 2, " ", false).is_none());
        assert!(after_insert("a", 1, "a", false).is_none());
    }
}
