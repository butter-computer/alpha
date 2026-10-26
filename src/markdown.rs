//! Markdown analysis for live rendering.
//!
//! The editor keeps raw markdown as the single source of truth. Block structure
//! (headings, lists, tasks, quotes, fences, rules) comes from a small line
//! scanner so it behaves deterministically while the user types. Inline spans
//! (bold, italic, code, strike, links) come from pulldown-cmark. Everything is
//! expressed as byte ranges into the same text so the conceal layer can hide
//! the syntax markers.

use pulldown_cmark::{Event, Parser, Tag};
use std::ops::Range;

pub const BOLD: u8 = 1;
pub const ITALIC: u8 = 2;
pub const CODE: u8 = 4;
pub const LINK: u8 = 8;
/// An inline delimiter (`**`, `` ` ``, `[`, `](url)`…) that is currently shown.
pub const MARKER: u8 = 16;
pub const STRIKE: u8 = 32;
/// Decoration prefix (bullet glyph, number, checkbox placeholder).
pub const PREFIX: u8 = 64;
/// Content of a checked task item.
pub const DONE: u8 = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Normal,
    Heading(u8),
    Quote,
    Code,
    Rule,
}

/// The block-level construct a line starts with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Lead {
    Heading { marker: Range<usize> },
    Bullet { marker: Range<usize>, depth: usize },
    Ordered { marker: Range<usize>, depth: usize, number: usize },
    Task { marker: Range<usize>, depth: usize, checked: bool, bracket: Range<usize> },
    Quote { marker: Range<usize> },
    Fence,
    Rule,
}

impl Lead {
    /// Buffer offset where the line's visible content begins.
    pub fn content_start(&self, line: &Line) -> usize {
        match self {
            Lead::Heading { marker }
            | Lead::Bullet { marker, .. }
            | Lead::Ordered { marker, .. }
            | Lead::Task { marker, .. }
            | Lead::Quote { marker } => marker.end,
            Lead::Fence | Lead::Rule => line.end,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub flags: u8,
}

/// An inline construct with delimiters that can be concealed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inline {
    pub full: Range<usize>,
    pub open: Range<usize>,
    pub close: Range<usize>,
    pub url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Line {
    /// Byte offset of the first character.
    pub start: usize,
    /// Byte offset one past the last character (excludes the newline).
    pub end: usize,
    pub kind: LineKind,
    pub lead: Option<Lead>,
    /// Style spans clipped to this line.
    pub spans: Vec<Span>,
    /// Inline delimiter ranges on this line, with the index of their `Inline`.
    pub delims: Vec<(Range<usize>, usize)>,
}

#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub lines: Vec<Line>,
    pub inlines: Vec<Inline>,
}

impl Analysis {
    /// Index of the line containing byte `offset`.
    pub fn line_at(&self, offset: usize) -> usize {
        match self.lines.binary_search_by(|l| l.start.cmp(&offset)) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }
}

// ---- block scanner -----------------------------------------------------------

fn heading_marker(trimmed: &str) -> Option<(u8, usize)> {
    let hashes = trimmed.bytes().take_while(|b| *b == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &trimmed[hashes..];
    if !(rest.starts_with(' ') || rest.starts_with('\t')) {
        return None;
    }
    let spaces = rest.bytes().take_while(|b| *b == b' ' || *b == b'\t').count();
    Some((hashes as u8, hashes + spaces))
}

fn is_rule(trimmed: &str) -> bool {
    let t = trimmed.trim_end();
    let Some(c) = t.chars().next() else { return false };
    matches!(c, '-' | '*' | '_') && t.len() >= 3 && t.chars().all(|x| x == c)
}

fn quote_prefix(line: &str) -> usize {
    let b = line.as_bytes();
    let mut i = 0;
    loop {
        let ws = b[i..].iter().take_while(|c| **c == b' ').count();
        if ws <= 3 && b.get(i + ws) == Some(&b'>') {
            i += ws + 1;
            if b.get(i) == Some(&b' ') {
                i += 1;
            }
        } else {
            return i;
        }
    }
}

enum ItemKind {
    Bullet,
    Ordered(usize),
}

struct Item {
    kind: ItemKind,
    /// Marker length (including one trailing space) relative to the trimmed line.
    marker_len: usize,
    /// `Some((bracket range relative to trimmed, checked))` for task items.
    task: Option<(Range<usize>, bool)>,
}

fn list_item(trimmed: &str) -> Option<Item> {
    let b = trimmed.as_bytes();
    let (kind, mut len) = if matches!(b.first(), Some(b'-' | b'*' | b'+')) && b.get(1) == Some(&b' ') {
        (ItemKind::Bullet, 2)
    } else {
        let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 || digits > 9 || !matches!(b.get(digits), Some(b'.' | b')')) || b.get(digits + 1) != Some(&b' ') {
            return None;
        }
        (ItemKind::Ordered(trimmed[..digits].parse().ok()?), digits + 2)
    };
    let mut task = None;
    if matches!(kind, ItemKind::Bullet) {
        let rest = &trimmed[len..];
        for (pat, checked) in [("[ ] ", false), ("[x] ", true), ("[X] ", true)] {
            if rest.starts_with(pat) {
                task = Some((len..len + 3, checked));
                len += 4;
                break;
            }
        }
    }
    Some(Item { kind, marker_len: len, task })
}

fn scan_lines(text: &str) -> Vec<Line> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut fence: Option<char> = None;
    // (indent, ordered counter) per open list level.
    let mut stack: Vec<(usize, Option<usize>)> = Vec::new();
    for l in text.split('\n') {
        let end = start + l.len();
        let mut line = Line { start, end, kind: LineKind::Normal, lead: None, spans: vec![], delims: vec![] };
        let trimmed = l.trim_start_matches(' ');
        let indent = l.len() - trimmed.len();
        let fence_ch = if trimmed.starts_with("```") {
            Some('`')
        } else if trimmed.starts_with("~~~") {
            Some('~')
        } else {
            None
        };
        if let Some(open) = fence {
            line.kind = LineKind::Code;
            if fence_ch == Some(open) && trimmed.trim_start_matches(open).trim().is_empty() {
                line.lead = Some(Lead::Fence);
                fence = None;
            }
        } else if let Some(ch) = fence_ch {
            line.kind = LineKind::Code;
            line.lead = Some(Lead::Fence);
            fence = Some(ch);
            stack.clear();
        } else if let Some((level, mlen)) = heading_marker(trimmed).filter(|_| indent < 4) {
            line.kind = LineKind::Heading(level);
            line.lead = Some(Lead::Heading { marker: start..start + indent + mlen });
            stack.clear();
        } else if is_rule(trimmed) && indent < 4 {
            line.kind = LineKind::Rule;
            line.lead = Some(Lead::Rule);
            stack.clear();
        } else if quote_prefix(l) > 0 {
            line.kind = LineKind::Quote;
            line.lead = Some(Lead::Quote { marker: start..start + quote_prefix(l) });
            stack.clear();
        } else if let Some(item) = list_item(trimmed) {
            while stack.last().is_some_and(|(i, _)| *i > indent) {
                stack.pop();
            }
            if stack.last().is_none_or(|(i, _)| *i != indent) {
                stack.push((indent, None));
            }
            let depth = stack.len() - 1;
            let marker = start..start + indent + item.marker_len;
            line.lead = Some(match (item.kind, item.task) {
                (_, Some((bracket, checked))) => {
                    stack[depth].1 = None;
                    Lead::Task {
                        marker,
                        depth,
                        checked,
                        bracket: start + indent + bracket.start..start + indent + bracket.end,
                    }
                }
                (ItemKind::Bullet, None) => {
                    stack[depth].1 = None;
                    Lead::Bullet { marker, depth }
                }
                (ItemKind::Ordered(n), None) => {
                    let number = stack[depth].1.map(|c| c + 1).unwrap_or(n);
                    stack[depth].1 = Some(number);
                    Lead::Ordered { marker, depth, number }
                }
            });
        } else if !l.trim().is_empty() {
            stack.clear();
        }
        out.push(line);
        start = end + 1;
    }
    out
}

// ---- analysis ---------------------------------------------------------------

pub fn analyze(text: &str) -> Analysis {
    let mut a = Analysis { lines: scan_lines(text), inlines: Vec::new() };
    let mut spans: Vec<Span> = Vec::new();

    for (event, range) in Parser::new_ext(text, crate::export::markdown_options()).into_offset_iter() {
        match event {
            Event::Start(Tag::Strong) => delimited(&mut spans, &mut a.inlines, &range, 2, BOLD),
            Event::Start(Tag::Emphasis) => delimited(&mut spans, &mut a.inlines, &range, 1, ITALIC),
            Event::Start(Tag::Strikethrough) => delimited(&mut spans, &mut a.inlines, &range, 2, STRIKE),
            Event::Code(_) => {
                let n = text[range.clone()].bytes().take_while(|b| *b == b'`').count();
                delimited(&mut spans, &mut a.inlines, &range, n, CODE);
            }
            Event::Start(Tag::Link { dest_url, .. }) | Event::Start(Tag::Image { dest_url, .. }) => {
                let src = &text[range.clone()];
                let open = if src.starts_with("![") { 2 } else if src.starts_with('[') { 1 } else { 0 };
                let url = Some(dest_url.to_string());
                match (open > 0).then(|| src.rfind("](")).flatten() {
                    Some(close) if close >= open => {
                        let (s, e) = (range.start, range.end);
                        spans.push(Span { range: s..s + open, flags: MARKER });
                        spans.push(Span { range: s + open..s + close, flags: LINK });
                        spans.push(Span { range: s + close..e, flags: MARKER });
                        a.inlines.push(Inline { full: range.clone(), open: s..s + open, close: s + close..e, url });
                    }
                    _ => {
                        spans.push(Span { range: range.clone(), flags: LINK });
                        a.inlines.push(Inline { full: range.clone(), open: range.start..range.start, close: range.end..range.end, url });
                    }
                }
            }
            _ => {}
        }
    }

    // Distribute spans to lines (never into code lines).
    for s in spans {
        if s.range.is_empty() {
            continue;
        }
        let l0 = a.line_at(s.range.start);
        let l1 = a.line_at(s.range.end.saturating_sub(1).max(s.range.start)).max(l0);
        for line in &mut a.lines[l0..=l1] {
            if line.kind == LineKind::Code {
                continue;
            }
            let r = s.range.start.max(line.start)..s.range.end.min(line.end);
            if r.start < r.end {
                line.spans.push(Span { range: r, flags: s.flags });
            }
        }
    }
    let inlines = std::mem::take(&mut a.inlines);
    let mut kept = Vec::new();
    for inl in inlines {
        let li = a.line_at(inl.full.start);
        if a.lines[li].kind == LineKind::Code {
            continue;
        }
        let idx = kept.len();
        for r in [&inl.open, &inl.close] {
            if !r.is_empty() {
                let l = a.line_at(r.start);
                a.lines[l].delims.push((r.clone(), idx));
            }
        }
        kept.push(inl);
    }
    a.inlines = kept;
    a
}

fn delimited(spans: &mut Vec<Span>, inlines: &mut Vec<Inline>, range: &Range<usize>, n: usize, flag: u8) {
    if n == 0 || range.len() < 2 * n {
        spans.push(Span { range: range.clone(), flags: flag });
        return;
    }
    let (s, e) = (range.start, range.end);
    spans.push(Span { range: s..s + n, flags: MARKER });
    spans.push(Span { range: s + n..e - n, flags: flag });
    spans.push(Span { range: e - n..e, flags: MARKER });
    inlines.push(Inline { full: range.clone(), open: s..s + n, close: e - n..e, url: None });
}

// ---- Enter key --------------------------------------------------------------

/// What pressing Enter at the end of a list/quote line should do.
#[derive(Debug, PartialEq, Eq)]
pub enum Continuation {
    /// Insert a newline followed by this prefix.
    Continue(String),
    /// The item was empty: remove its marker (`n` leading bytes) instead.
    Clear(usize),
}

pub fn continuation(line: &str) -> Option<Continuation> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let indent = &line[..indent_len];
    let rest = &line[indent_len..];
    let (marker_len, next_marker) = if let Some(c) = rest.chars().next().filter(|c| matches!(c, '-' | '*' | '+' | '>')) {
        (c.len_utf8(), c.to_string())
    } else {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        let delim = rest[digits..].chars().next()?;
        if digits == 0 || digits > 9 || !matches!(delim, '.' | ')') {
            return None;
        }
        let n: u64 = rest[..digits].parse().ok()?;
        (digits + 1, format!("{}{}", n + 1, delim))
    };
    let after = &rest[marker_len..];
    if !after.starts_with(' ') {
        return None;
    }
    let mut content = after.trim();
    let mut task = false;
    if next_marker != ">" {
        for pat in ["[ ]", "[x]", "[X]"] {
            if let Some(r) = content.strip_prefix(pat)
                && (r.is_empty() || r.starts_with(' '))
            {
                content = r.trim();
                task = true;
                break;
            }
        }
    }
    if content.is_empty() {
        return Some(Continuation::Clear(line.len()));
    }
    let boxed = if task { "[ ] " } else { "" };
    Some(Continuation::Continue(format!("{indent}{next_marker} {boxed}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lead(t: &str, line: usize) -> Option<Lead> {
        analyze(t).lines[line].lead.clone()
    }

    #[test]
    fn headings() {
        let a = analyze("# One\n\n### Three\n#### Four\n#nospace\nplain");
        assert_eq!(a.lines[0].kind, LineKind::Heading(1));
        assert_eq!(a.lines[0].lead, Some(Lead::Heading { marker: 0..2 }));
        assert_eq!(a.lines[2].kind, LineKind::Heading(3));
        assert_eq!(a.lines[3].kind, LineKind::Heading(4));
        assert_eq!(a.lines[4].kind, LineKind::Normal);
        assert_eq!(a.lines[5].kind, LineKind::Normal);
    }

    #[test]
    fn empty_heading_with_only_marker() {
        let a = analyze("# ");
        assert_eq!(a.lines[0].kind, LineKind::Heading(1));
        assert_eq!(a.lines[0].lead.as_ref().unwrap().content_start(&a.lines[0]), 2);
    }

    #[test]
    fn bullets_numbers_and_tasks() {
        let t = "- a\n* b\n1. c\n2. d\n- [ ] e\n- [x] f\n- [X] g";
        assert_eq!(lead(t, 0), Some(Lead::Bullet { marker: 0..2, depth: 0 }));
        assert!(matches!(lead(t, 1), Some(Lead::Bullet { .. })));
        assert_eq!(lead(t, 2), Some(Lead::Ordered { marker: 8..11, depth: 0, number: 1 }));
        assert!(matches!(lead(t, 3), Some(Lead::Ordered { number: 2, .. })));
        match lead(t, 4) {
            Some(Lead::Task { marker, checked, bracket, .. }) => {
                assert!(!checked);
                assert_eq!(&t[marker], "- [ ] ");
                assert_eq!(&t[bracket], "[ ]");
            }
            o => panic!("{o:?}"),
        }
        assert!(matches!(lead(t, 5), Some(Lead::Task { checked: true, .. })));
        assert!(matches!(lead(t, 6), Some(Lead::Task { checked: true, .. })));
    }

    #[test]
    fn ordered_numbers_are_renumbered_and_nested() {
        let t = "1. a\n1. b\n    - x\n    - y\n1. c";
        assert!(matches!(lead(t, 0), Some(Lead::Ordered { number: 1, depth: 0, .. })));
        assert!(matches!(lead(t, 1), Some(Lead::Ordered { number: 2, depth: 0, .. })));
        assert!(matches!(lead(t, 2), Some(Lead::Bullet { depth: 1, .. })));
        assert!(matches!(lead(t, 3), Some(Lead::Bullet { depth: 1, .. })));
        assert!(matches!(lead(t, 4), Some(Lead::Ordered { number: 3, depth: 0, .. })));
    }

    #[test]
    fn list_item_requires_space() {
        assert_eq!(lead("-no", 0), None);
        assert_eq!(lead("1.no", 0), None);
        assert_eq!(lead("- [ ]x", 0), Some(Lead::Bullet { marker: 0..2, depth: 0 }));
    }

    #[test]
    fn quotes_rules_and_fences() {
        let t = "> q\n---\n```\ncode\n```\nafter";
        let a = analyze(t);
        assert_eq!(a.lines[0].kind, LineKind::Quote);
        assert_eq!(a.lines[0].lead, Some(Lead::Quote { marker: 0..2 }));
        assert_eq!(a.lines[1].kind, LineKind::Rule);
        assert_eq!(a.lines[2].lead, Some(Lead::Fence));
        assert_eq!(a.lines[3].kind, LineKind::Code);
        assert_eq!(a.lines[3].lead, None);
        assert_eq!(a.lines[4].lead, Some(Lead::Fence));
        assert_eq!(a.lines[5].kind, LineKind::Normal);
    }

    #[test]
    fn unclosed_fence_runs_to_end() {
        let a = analyze("```\na\nb");
        assert!(a.lines.iter().all(|l| l.kind == LineKind::Code));
    }

    #[test]
    fn inline_spans_and_delims() {
        let t = "a **b** *i* `c` ~~s~~";
        let a = analyze(t);
        let line = &a.lines[0];
        let flag_at = |needle: &str| {
            let o = t.find(needle).unwrap();
            line.spans.iter().filter(|s| s.range.contains(&o)).fold(0, |f, s| f | s.flags)
        };
        assert_eq!(flag_at("b"), BOLD);
        assert_eq!(flag_at("i"), ITALIC);
        assert_eq!(flag_at("c"), CODE);
        assert_eq!(flag_at("s"), STRIKE);
        assert_eq!(a.inlines.len(), 4);
        assert_eq!(&t[a.inlines[0].open.clone()], "**");
        assert_eq!(&t[a.inlines[0].close.clone()], "**");
        assert_eq!(line.delims.len(), 8);
    }

    #[test]
    fn links_expose_url_and_delims() {
        let t = "see [text](http://x.y) ok";
        let a = analyze(t);
        assert_eq!(a.inlines[0].url.as_deref(), Some("http://x.y"));
        assert_eq!(&t[a.inlines[0].open.clone()], "[");
        assert_eq!(&t[a.inlines[0].close.clone()], "](http://x.y)");
    }

    #[test]
    fn no_inline_in_code_blocks() {
        let a = analyze("```\n**x**\n```");
        assert!(a.inlines.is_empty());
        assert!(a.lines[1].spans.is_empty());
    }

    #[test]
    fn line_at_and_coverage() {
        let a = analyze("a\n\nbc\n");
        assert_eq!(a.lines.len(), 4);
        assert_eq!((a.lines[2].start, a.lines[2].end), (3, 5));
        assert_eq!(a.line_at(4), 2);
        assert_eq!(a.line_at(6), 3);
    }

    #[test]
    fn enter_in_lists() {
        assert_eq!(continuation("- foo"), Some(Continuation::Continue("- ".into())));
        assert_eq!(continuation("  * foo"), Some(Continuation::Continue("  * ".into())));
        assert_eq!(continuation("3. foo"), Some(Continuation::Continue("4. ".into())));
        assert_eq!(continuation("> hi"), Some(Continuation::Continue("> ".into())));
        assert_eq!(continuation("- [x] done"), Some(Continuation::Continue("- [ ] ".into())));
        assert_eq!(continuation("- [ ] todo"), Some(Continuation::Continue("- [ ] ".into())));
        assert_eq!(continuation("- "), Some(Continuation::Clear(2)));
        assert_eq!(continuation("- [ ] "), Some(Continuation::Clear(6)));
        assert_eq!(continuation("1. "), Some(Continuation::Clear(3)));
        assert_eq!(continuation("plain"), None);
        assert_eq!(continuation("-no"), None);
    }
}
