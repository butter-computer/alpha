//! Markdown analysis for live rendering.
//!
//! The editor keeps the raw markdown text as the single source of truth. This
//! module parses it with pulldown-cmark and produces per-line kinds plus inline
//! style spans (as byte ranges into the same text) so the editor can style
//! markers and content in place.

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};
use std::ops::Range;

pub const BOLD: u8 = 1;
pub const ITALIC: u8 = 2;
pub const CODE: u8 = 4;
pub const LINK: u8 = 8;
/// Syntax characters (`**`, `#`, `>`, fences, link urls…) that can be dimmed.
pub const MARKER: u8 = 16;
pub const STRIKE: u8 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Normal,
    Heading(u8),
    Quote,
    Code,
    Rule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub flags: u8,
}

#[derive(Clone, Debug)]
pub struct Line {
    /// Byte offset of the first character.
    pub start: usize,
    /// Byte offset one past the last character (excludes the newline).
    pub end: usize,
    pub kind: LineKind,
    /// Spans clipped to this line.
    pub spans: Vec<Span>,
}

#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub lines: Vec<Line>,
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

impl Line {
    /// Contiguous segments covering the whole line, each with merged flags.
    pub fn segments(&self) -> Vec<(Range<usize>, u8)> {
        if self.start == self.end {
            return Vec::new();
        }
        let mut cuts = vec![self.start, self.end];
        for s in &self.spans {
            cuts.push(s.range.start);
            cuts.push(s.range.end);
        }
        cuts.sort_unstable();
        cuts.dedup();
        cuts.windows(2)
            .map(|w| {
                let flags = self
                    .spans
                    .iter()
                    .filter(|s| s.range.start <= w[0] && s.range.end >= w[1])
                    .fold(0, |acc, s| acc | s.flags);
                (w[0]..w[1], flags)
            })
            .collect()
    }
}

pub fn analyze(text: &str) -> Analysis {
    let mut lines = Vec::new();
    let mut start = 0;
    for l in text.split('\n') {
        lines.push(Line { start, end: start + l.len(), kind: LineKind::Normal, spans: Vec::new() });
        start += l.len() + 1;
    }
    let mut a = Analysis { lines };
    let mut spans: Vec<Span> = Vec::new();

    let opts = crate::export::markdown_options();
    for (event, range) in Parser::new_ext(text, opts).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                set_kind(&mut a, &range, LineKind::Heading(level as u8));
                let src = &text[range.clone()];
                if src.starts_with('#') {
                    let hashes = src.bytes().take_while(|b| *b == b'#').count();
                    let spaces = src[hashes..].bytes().take_while(|b| *b == b' ' || *b == b'\t').count();
                    mark(&mut spans, range.start..range.start + hashes + spaces);
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                set_kind(&mut a, &range, LineKind::Quote);
                let (l0, l1) = line_span(&a, &range);
                for line in &a.lines[l0..=l1] {
                    let src = &text[line.start..line.end];
                    let mut i = 0;
                    let b = src.as_bytes();
                    loop {
                        let ws = b[i..].iter().take_while(|c| **c == b' ').count();
                        if b.get(i + ws) == Some(&b'>') {
                            i += ws + 1;
                            if b.get(i) == Some(&b' ') {
                                i += 1;
                            }
                        } else {
                            break;
                        }
                    }
                    if i > 0 {
                        mark(&mut spans, line.start..line.start + i);
                    }
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                set_kind(&mut a, &range, LineKind::Code);
                if matches!(kind, CodeBlockKind::Fenced(_)) {
                    let (l0, l1) = line_span(&a, &range);
                    let is_fence = |l: &Line| {
                        let t = text[l.start..l.end].trim_start();
                        t.starts_with("```") || t.starts_with("~~~")
                    };
                    if is_fence(&a.lines[l0]) {
                        mark(&mut spans, a.lines[l0].start..a.lines[l0].end);
                        if l1 > l0 && is_fence(&a.lines[l1]) {
                            mark(&mut spans, a.lines[l1].start..a.lines[l1].end);
                        }
                    }
                }
            }
            Event::Rule => set_kind(&mut a, &range, LineKind::Rule),
            Event::Start(Tag::Item) => {
                let len = text[range.start..]
                    .bytes()
                    .take_while(|b| !b.is_ascii_whitespace())
                    .count();
                mark(&mut spans, range.start..range.start + len);
            }
            Event::Start(Tag::Strong) => delimited(&mut spans, &range, 2, BOLD),
            Event::Start(Tag::Emphasis) => delimited(&mut spans, &range, 1, ITALIC),
            Event::Start(Tag::Strikethrough) => delimited(&mut spans, &range, 2, STRIKE),
            Event::Code(_) => {
                let src = &text[range.clone()];
                let n = src.bytes().take_while(|b| *b == b'`').count();
                delimited(&mut spans, &range, n, CODE);
            }
            Event::Start(Tag::Link { .. }) | Event::Start(Tag::Image { .. }) => {
                let src = &text[range.clone()];
                let open = if src.starts_with("![") { 2 } else if src.starts_with('[') { 1 } else { 0 };
                match (open > 0).then(|| src.rfind("](")).flatten() {
                    Some(close) if close >= open => {
                        mark(&mut spans, range.start..range.start + open);
                        spans.push(Span { range: range.start + open..range.start + close, flags: LINK });
                        mark(&mut spans, range.start + close..range.end);
                    }
                    _ => spans.push(Span { range: range.clone(), flags: LINK }),
                }
            }
            _ => {}
        }
    }

    for s in spans {
        if s.range.is_empty() {
            continue;
        }
        let (l0, l1) = line_span(&a, &s.range);
        for line in &mut a.lines[l0..=l1] {
            let r = s.range.start.max(line.start)..s.range.end.min(line.end);
            if r.start < r.end {
                line.spans.push(Span { range: r, flags: s.flags });
            }
        }
    }
    a
}

fn mark(spans: &mut Vec<Span>, range: Range<usize>) {
    spans.push(Span { range, flags: MARKER });
}

fn delimited(spans: &mut Vec<Span>, range: &Range<usize>, n: usize, flag: u8) {
    if n == 0 || range.len() < 2 * n {
        spans.push(Span { range: range.clone(), flags: flag });
        return;
    }
    mark(spans, range.start..range.start + n);
    spans.push(Span { range: range.start + n..range.end - n, flags: flag });
    mark(spans, range.end - n..range.end);
}

fn line_span(a: &Analysis, range: &Range<usize>) -> (usize, usize) {
    let l0 = a.line_at(range.start);
    let l1 = a.line_at(range.end.saturating_sub(1).max(range.start));
    (l0, l1.max(l0))
}

fn set_kind(a: &mut Analysis, range: &Range<usize>, kind: LineKind) {
    let (l0, l1) = line_span(a, range);
    for line in &mut a.lines[l0..=l1] {
        // Don't let an outer quote overwrite a more specific inner block.
        if kind != LineKind::Quote || line.kind == LineKind::Normal {
            line.kind = kind;
        }
    }
}

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
    let content = after.trim();
    if content.is_empty() {
        return Some(Continuation::Clear(line.len()));
    }
    Some(Continuation::Continue(format!("{indent}{next_marker} ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags_at(a: &Analysis, text: &str, needle: &str) -> u8 {
        let off = text.find(needle).unwrap();
        let line = &a.lines[a.line_at(off)];
        line.segments()
            .into_iter()
            .find(|(r, _)| r.start <= off && off < r.end)
            .map(|(_, f)| f)
            .unwrap()
    }

    #[test]
    fn headings_have_level_and_dim_marker() {
        let t = "# One\n\n### Three\nplain";
        let a = analyze(t);
        assert_eq!(a.lines[0].kind, LineKind::Heading(1));
        assert_eq!(a.lines[1].kind, LineKind::Normal);
        assert_eq!(a.lines[2].kind, LineKind::Heading(3));
        assert_eq!(a.lines[3].kind, LineKind::Normal);
        assert_eq!(flags_at(&a, t, "# One") & MARKER, MARKER);
        assert_eq!(flags_at(&a, t, "One") & MARKER, 0);
        assert_eq!(a.lines[0].segments()[0].0, 0..2);
    }

    #[test]
    fn bold_italic_code_spans() {
        let t = "a **b** *i* `c` ~~s~~";
        let a = analyze(t);
        assert_eq!(flags_at(&a, t, "b"), BOLD);
        assert_eq!(flags_at(&a, t, "i"), ITALIC);
        assert_eq!(flags_at(&a, t, "c"), CODE);
        assert_eq!(flags_at(&a, t, "s"), STRIKE);
        assert_eq!(flags_at(&a, t, "**b"), MARKER);
        assert_eq!(flags_at(&a, t, "a"), 0);
    }

    #[test]
    fn nested_bold_italic_merges() {
        let t = "***x***";
        let a = analyze(t);
        assert_eq!(flags_at(&a, t, "x") & (BOLD | ITALIC), BOLD | ITALIC);
    }

    #[test]
    fn links_dim_url() {
        let t = "see [text](http://x.y) ok";
        let a = analyze(t);
        assert_eq!(flags_at(&a, t, "text"), LINK);
        assert_eq!(flags_at(&a, t, "](http"), MARKER);
        assert_eq!(flags_at(&a, t, "ok"), 0);
    }

    #[test]
    fn quotes_rules_and_lists() {
        let t = "> quoted\n\n---\n\n- item\n1. one";
        let a = analyze(t);
        assert_eq!(a.lines[0].kind, LineKind::Quote);
        assert_eq!(flags_at(&a, t, "> q"), MARKER);
        assert_eq!(flags_at(&a, t, "quoted"), 0);
        assert_eq!(a.lines[2].kind, LineKind::Rule);
        assert_eq!(flags_at(&a, t, "- item"), MARKER);
        assert_eq!(flags_at(&a, t, "item"), 0);
        assert_eq!(flags_at(&a, t, "1."), MARKER);
    }

    #[test]
    fn fenced_code_blocks() {
        let t = "```rust\nlet **x** = 1;\n```\nafter";
        let a = analyze(t);
        assert_eq!(a.lines[0].kind, LineKind::Code);
        assert_eq!(a.lines[1].kind, LineKind::Code);
        assert_eq!(a.lines[2].kind, LineKind::Code);
        assert_eq!(a.lines[3].kind, LineKind::Normal);
        assert_eq!(flags_at(&a, t, "```rust"), MARKER);
        // Inline markdown inside code is not parsed.
        assert_eq!(flags_at(&a, t, "**x**"), 0);
    }

    #[test]
    fn lines_cover_text() {
        let t = "a\n\nbc\n";
        let a = analyze(t);
        assert_eq!(a.lines.len(), 4);
        assert_eq!((a.lines[2].start, a.lines[2].end), (3, 5));
        assert_eq!(a.line_at(4), 2);
        assert_eq!(a.line_at(6), 3);
    }

    #[test]
    fn list_continuation() {
        assert_eq!(continuation("- foo"), Some(Continuation::Continue("- ".into())));
        assert_eq!(continuation("  * foo"), Some(Continuation::Continue("  * ".into())));
        assert_eq!(continuation("3. foo"), Some(Continuation::Continue("4. ".into())));
        assert_eq!(continuation("> hi"), Some(Continuation::Continue("> ".into())));
        assert_eq!(continuation("- "), Some(Continuation::Clear(2)));
        assert_eq!(continuation("plain"), None);
        assert_eq!(continuation("-no"), None);
        assert_eq!(continuation("12.x"), None);
    }
}
