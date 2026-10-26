//! The `/` block menu.

use crate::fuzzy;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    Text,
    H1,
    H2,
    H3,
    Bullet,
    Numbered,
    Checklist,
    Quote,
    Code,
    Divider,
}

pub struct Item {
    pub label: &'static str,
    pub hint: &'static str,
    pub block: Block,
}

pub const ITEMS: &[Item] = &[
    Item { label: "Text", hint: "", block: Block::Text },
    Item { label: "Heading 1", hint: "#", block: Block::H1 },
    Item { label: "Heading 2", hint: "##", block: Block::H2 },
    Item { label: "Heading 3", hint: "###", block: Block::H3 },
    Item { label: "Bullet list", hint: "-", block: Block::Bullet },
    Item { label: "Numbered list", hint: "1.", block: Block::Numbered },
    Item { label: "Checklist", hint: "[]", block: Block::Checklist },
    Item { label: "Quote", hint: ">", block: Block::Quote },
    Item { label: "Code block", hint: "```", block: Block::Code },
    Item { label: "Divider", hint: "---", block: Block::Divider },
];

/// Whether the line above `ls` has content (a divider typed right under it
/// would otherwise read as a setext underline in exported HTML).
fn prev_line_nonblank(text: &str, ls: usize) -> bool {
    ls > 0 && text[..ls - 1].rsplit('\n').next().is_some_and(|l| !l.trim().is_empty())
}

pub fn filter(query: &str) -> Vec<&'static Item> {
    fuzzy::filter(query, ITEMS, |i| i.label)
}

/// Whether a just-typed `/` at `caret` should open the menu: the slash is the
/// first non-space character of its line.
pub fn should_open(text: &str, caret: usize) -> bool {
    if caret == 0 || !text[..caret].ends_with('/') {
        return false;
    }
    let ls = text[..caret].rfind('\n').map(|i| i + 1).unwrap_or(0);
    text[ls..caret - 1].chars().all(|c| c == ' ' || c == '\t')
}

/// The filter text typed after the `/` at `start`, or `None` if the menu should close.
pub fn query(text: &str, start: usize, caret: usize) -> Option<&str> {
    if caret <= start || !text.get(start..)?.starts_with('/') || caret > text.len() {
        return None;
    }
    let q = text.get(start + 1..caret)?;
    (!q.contains(char::is_whitespace) && q.len() <= 24).then_some(q)
}

/// Replacement for `/query` and the caret offset inside it. Identical to what
/// typing the markdown by hand produces.
pub fn replacement(block: Block, text: &str, slash_at: usize) -> (String, usize) {
    let ls = text[..slash_at].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let simple = |s: &str| (s.to_string(), s.len());
    match block {
        Block::Text => simple(""),
        Block::H1 => simple("# "),
        Block::H2 => simple("## "),
        Block::H3 => simple("### "),
        Block::Bullet => simple("- "),
        Block::Numbered => simple("1. "),
        Block::Checklist => simple("- [ ] "),
        Block::Quote => simple("> "),
        Block::Code => ("```\n\n```".to_string(), 4),
        Block::Divider => {
            if prev_line_nonblank(text, ls) {
                ("\n---\n".to_string(), 5)
            } else {
                ("---\n".to_string(), 4)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opens_only_at_line_start() {
        assert!(should_open("/", 1));
        assert!(should_open("a\n  /", 5));
        assert!(!should_open("a/", 2));
        assert!(!should_open("a /", 3));
        assert!(!should_open("ab", 2));
    }

    #[test]
    fn query_tracks_typing_and_closes() {
        assert_eq!(query("/hea", 0, 4), Some("hea"));
        assert_eq!(query("/", 0, 1), Some(""));
        assert_eq!(query("/a b", 0, 4), None);
        assert_eq!(query("x", 0, 1), None);
        assert_eq!(query("/a", 1, 2), None);
    }

    #[test]
    fn filter_is_fuzzy_and_lists_all_blocks() {
        assert_eq!(filter("").len(), 10);
        assert_eq!(filter("h2")[0].block, Block::H2);
        assert_eq!(filter("chk").first().map(|i| i.block), Some(Block::Checklist));
        assert_eq!(filter("code")[0].block, Block::Code);
        assert!(filter("zzzz").is_empty());
    }

    #[test]
    fn replacements_match_typed_markdown() {
        assert_eq!(replacement(Block::H1, "/", 0), ("# ".into(), 2));
        assert_eq!(replacement(Block::Bullet, "/", 0), ("- ".into(), 2));
        assert_eq!(replacement(Block::Checklist, "/", 0), ("- [ ] ".into(), 6));
        assert_eq!(replacement(Block::Code, "/", 0), ("```\n\n```".into(), 4));
        assert_eq!(replacement(Block::Text, "/", 0), ("".into(), 0));
        assert_eq!(replacement(Block::Divider, "x\n/", 2).0, "\n---\n");
        assert_eq!(replacement(Block::Divider, "/", 0).0, "---\n");
    }
}
