//! Markdown analysis for live rendering.
//!
//! The editor keeps raw markdown as the single source of truth and `analyze` is
//! a pure function of that text (plus the caret line, see below). Block and
//! inline structure both come from pulldown-cmark's offset iterator, so a line
//! that reads `# Hi` is a heading no matter how it got there. Everything is
//! expressed as byte ranges into the same text so the conceal layer can hide
//! the syntax markers.
//!
//! A thin line-level layer covers what a CommonMark parser deliberately does not
//! give an editor:
//! * markers are only "complete" once followed by a space (`#` alone is text);
//! * setext headings are not a thing (`text` over `---` is a paragraph and a
//!   divider), and
//! * an empty `- ` / `1. ` directly under a paragraph is still a list item.
//!
//! The one caret-dependent rule: a fence the caret is typing on that has no
//! closing fence yet is treated as plain text, so typing ``` above existing
//! content does not turn the rest of the document into code before Enter.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
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

/// The block-level construct a line starts with. Marker ranges start at the
/// line start (they include indentation and any quote prefix).
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

    fn rank(&self) -> u8 {
        match self {
            Lead::Quote { .. } => 1,
            Lead::Heading { .. } => 2,
            Lead::Bullet { .. } | Lead::Ordered { .. } | Lead::Task { .. } => 3,
            Lead::Fence | Lead::Rule => 4,
        }
    }

    pub fn is_list(&self) -> bool {
        matches!(self, Lead::Bullet { .. } | Lead::Ordered { .. } | Lead::Task { .. })
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeRole {
    Open { lang: String },
    Body,
    Close,
}

/// A line that belongs to a fenced code block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeLine {
    /// Identifies the block (byte offset of its opening line).
    pub block: usize,
    pub role: CodeRole,
}

#[derive(Clone, Debug)]
pub struct Line {
    /// Byte offset of the first character.
    pub start: usize,
    /// Byte offset one past the last character (excludes the newline).
    pub end: usize,
    pub kind: LineKind,
    pub lead: Option<Lead>,
    pub code: Option<CodeLine>,
    /// Style spans clipped to this line.
    pub spans: Vec<Span>,
    /// Inline delimiter ranges on this line, with the index of their `Inline`.
    pub delims: Vec<(Range<usize>, usize)>,
}

impl Line {
    /// The range hidden by the block marker (a divider's marker is its whole line).
    pub fn marker(&self) -> Option<Range<usize>> {
        match self.lead.as_ref()? {
            Lead::Heading { marker }
            | Lead::Bullet { marker, .. }
            | Lead::Ordered { marker, .. }
            | Lead::Task { marker, .. }
            | Lead::Quote { marker } => Some(marker.clone()),
            Lead::Rule => Some(self.start..self.end),
            Lead::Fence => None,
        }
    }

    pub fn is_fence(&self) -> bool {
        matches!(self.lead, Some(Lead::Fence))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Analysis {
    pub lines: Vec<Line>,
    pub inlines: Vec<Inline>,
    /// Indices of lines that open a fenced block which never closes.
    pub unclosed: Vec<usize>,
}

impl Analysis {
    /// Index of the line containing byte `offset`.
    pub fn line_at(&self, offset: usize) -> usize {
        match self.lines.binary_search_by(|l| l.start.cmp(&offset)) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }

    /// The (open, close) line indices of the fenced block containing line `i`.
    /// `close` is `None` while the block is unclosed.
    pub fn block_lines(&self, i: usize) -> Option<(usize, Option<usize>)> {
        let id = self.lines.get(i)?.code.as_ref()?.block;
        let open = self.line_at(id);
        let mut close = None;
        let mut j = i;
        while j < self.lines.len() && self.lines[j].code.as_ref().is_some_and(|c| c.block == id) {
            if self.lines[j].code.as_ref().is_some_and(|c| c.role == CodeRole::Close) {
                close = Some(j);
            }
            j += 1;
        }
        Some((open, close))
    }
}

// ---- line scanning helpers ----------------------------------------------------

fn parser_options() -> Options {
    Options::ENABLE_STRIKETHROUGH
}

fn split_lines(text: &str) -> Vec<Line> {
    let mut out = Vec::new();
    let mut start = 0;
    for l in text.split('\n') {
        let end = start + l.len();
        out.push(Line { start, end, kind: LineKind::Normal, lead: None, code: None, spans: vec![], delims: vec![] });
        start = end + 1;
    }
    out
}

/// A list marker found at the start of `s` (after indentation).
struct ItemMarker {
    /// Marker length including the following spaces.
    len: usize,
    ordered: Option<usize>,
    task: Option<(Range<usize>, bool)>,
}

/// Parses `-`/`*`/`+`/`1.`/`1)` plus the spaces after it, and a task box. The
/// marker is only complete when a space follows it.
fn item_marker(s: &str) -> Option<ItemMarker> {
    let b = s.as_bytes();
    let (mlen, ordered) = if matches!(b.first(), Some(b'-' | b'*' | b'+')) {
        (1, None)
    } else {
        let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 || digits > 9 || !matches!(b.get(digits), Some(b'.' | b')')) {
            return None;
        }
        (digits + 1, Some(s[..digits].parse().ok()?))
    };
    if !matches!(b.get(mlen), Some(b' ' | b'\t')) {
        return None;
    }
    let spaces = b[mlen..].iter().take_while(|c| **c == b' ' || **c == b'\t').count();
    let rest_blank = mlen + spaces == b.len();
    let eaten = if rest_blank || spaces <= 4 { spaces } else { 1 };
    let mut len = mlen + eaten;
    let mut task = None;
    if ordered.is_none() && !rest_blank {
        let rest = &s[len..];
        for (pat, checked) in [("[ ] ", false), ("[x] ", true), ("[X] ", true)] {
            if rest.starts_with(pat) {
                task = Some((len..len + 3, checked));
                len += 4;
                break;
            }
        }
    }
    Some(ItemMarker { len, ordered, task })
}

/// Length of the `>` prefix (`> `, `> > `, …) at the start of `line`.
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

fn is_closing_fence(line: &str, ch: u8, n: usize) -> bool {
    let t = line.trim_start_matches(' ');
    let run = t.bytes().take_while(|b| *b == ch).count();
    run >= n && t[run..].trim().is_empty()
}

fn set_lead(line: &mut Line, lead: Lead) {
    if line.lead.as_ref().is_none_or(|l| l.rank() <= lead.rank()) {
        line.lead = Some(lead);
    }
}

// ---- analysis ---------------------------------------------------------------

/// Analyzes `text`. `caret` only matters for one case: an unclosed fence being
/// typed on the caret's line is treated as plain text (see the module docs).
pub fn analyze(text: &str, caret: usize) -> Analysis {
    let a = parse(text);
    let ci = a.line_at(caret.min(text.len()));
    if a.unclosed.contains(&ci) {
        return parse(&neutralize_line(text, &a.lines[ci]));
    }
    a
}

/// Analysis with the line at `line_start` forced to plain text (fence characters
/// replaced by same-length ASCII so every offset stays valid).
pub fn analyze_plain_line(text: &str, line_start: usize) -> Analysis {
    let a = parse(text);
    let l = &a.lines[a.line_at(line_start)];
    parse(&neutralize_line(text, l))
}

fn neutralize_line(text: &str, line: &Line) -> String {
    let mut b = text.as_bytes().to_vec();
    for x in &mut b[line.start..line.end] {
        if matches!(*x, b'`' | b'~') {
            *x = b'x';
        }
    }
    String::from_utf8(b).expect("ASCII-for-ASCII substitution keeps UTF-8 valid")
}

fn parse(text: &str) -> Analysis {
    let mut a = Analysis { lines: split_lines(text), ..Default::default() };
    let mut spans: Vec<Span> = Vec::new();
    let mut in_quote = vec![false; a.lines.len()];
    // Next number to hand out per open list (`None` for bullet lists).
    let mut lists: Vec<Option<usize>> = Vec::new();

    for (event, range) in Parser::new_ext(text, parser_options()).into_offset_iter() {
        match event {
            Event::Start(Tag::List(first)) => lists.push(first.map(|n| n as usize)),
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::Item) => {
                let depth = lists.len().saturating_sub(1);
                let number = lists.last_mut().and_then(|n| n.as_mut()).map(|n| {
                    let v = *n;
                    *n += 1;
                    v
                });
                let li = a.line_at(range.start);
                let line = &mut a.lines[li];
                let from = range.start.max(line.start).min(line.end);
                let skip = text[from..line.end].bytes().take_while(|b| *b == b' ' || *b == b'\t').count();
                let at = from + skip;
                if let Some(m) = item_marker(&text[at..line.end]) {
                    let marker = line.start..at + m.len;
                    let lead = match (m.task, m.ordered) {
                        (Some((b, checked)), _) => Lead::Task { marker, depth, checked, bracket: at + b.start..at + b.end },
                        (None, Some(n)) => Lead::Ordered { marker, depth, number: number.unwrap_or(n) },
                        (None, None) => Lead::Bullet { marker, depth },
                    };
                    set_lead(line, lead);
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let li = a.line_at(range.start);
                let src = &text[range.start..a.lines[li].end];
                if src.starts_with('#') {
                    let hashes = src.bytes().take_while(|b| *b == b'#').count();
                    let spaces = src[hashes..].bytes().take_while(|b| *b == b' ' || *b == b'\t').count();
                    if spaces > 0 {
                        let marker = a.lines[li].start..range.start + hashes + spaces;
                        set_lead(&mut a.lines[li], Lead::Heading { marker });
                        a.lines[li].kind = LineKind::Heading(level as u8);
                    }
                } else {
                    // Setext: only the divider underline means something.
                    let ul = a.line_at(range.end.saturating_sub(1).max(range.start));
                    let l = &a.lines[ul];
                    let t = text[l.start..l.end].trim();
                    if ul > li && t.len() >= 3 && t.bytes().all(|b| b == b'-') && text[l.start..l.end].len() - text[l.start..l.end].trim_start().len() < 4 {
                        let line = &mut a.lines[ul];
                        line.lead = Some(Lead::Rule);
                    }
                }
            }
            Event::Rule => {
                let li = a.line_at(range.start);
                let l = &a.lines[li];
                if text[l.start..range.start].bytes().all(|b| b == b' ') && text[range.end.min(l.end)..l.end].trim().is_empty() {
                    a.lines[li].lead = Some(Lead::Rule);
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                let l0 = a.line_at(range.start);
                let l1 = a.line_at(range.end.saturating_sub(1).max(range.start)).max(l0);
                let first = &a.lines[l0];
                let p0 = quote_prefix(&text[first.start..first.end]);
                let complete = p0 > 0 && (text[first.start..first.start + p0].ends_with(' ') || first.start + p0 < first.end);
                if !complete {
                    continue;
                }
                for (line, quoted) in a.lines[l0..=l1].iter_mut().zip(&mut in_quote[l0..=l1]) {
                    *quoted = true;
                    let p = quote_prefix(&text[line.start..line.end]);
                    if p > 0 {
                        set_lead(line, Lead::Quote { marker: line.start..line.start + p });
                    }
                }
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => fence(text, &mut a, &range, &info),
            Event::Start(Tag::Strong) => delimited(&mut spans, &mut a.inlines, &range, 2, BOLD),
            Event::Start(Tag::Emphasis) => delimited(&mut spans, &mut a.inlines, &range, 1, ITALIC),
            Event::Start(Tag::Strikethrough) => {
                // `~x~` and `~~x~~` are both strikethrough.
                let n = text[range.clone()].bytes().take_while(|b| *b == b'~').count();
                delimited(&mut spans, &mut a.inlines, &range, n, STRIKE);
            }
            Event::Code(_) => {
                let n = text[range.clone()].bytes().take_while(|b| *b == b'`').count();
                delimited(&mut spans, &mut a.inlines, &range, n, CODE);
            }
            Event::Start(Tag::Link { dest_url, .. }) | Event::Start(Tag::Image { dest_url, .. }) => {
                let src = &text[range.clone()];
                let open = if src.starts_with("![") {
                    2
                } else if src.starts_with('[') {
                    1
                } else {
                    0
                };
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

    fallback_items(text, &mut a);

    for (i, line) in a.lines.iter_mut().enumerate() {
        line.kind = if line.code.is_some() {
            LineKind::Code
        } else {
            match line.lead {
                Some(Lead::Rule) => LineKind::Rule,
                _ if matches!(line.kind, LineKind::Heading(_)) => line.kind,
                _ if in_quote[i] => LineKind::Quote,
                _ => LineKind::Normal,
            }
        };
    }

    distribute(&mut a, spans);
    a
}

fn fence(text: &str, a: &mut Analysis, range: &Range<usize>, info: &str) {
    let l0 = a.line_at(range.start);
    let (s0, e0) = (a.lines[l0].start, a.lines[l0].end);
    let opener_ok = text[s0..range.start].bytes().all(|b| b == b' ');
    let src = &text[range.start..e0];
    let ch = src.as_bytes().first().copied().unwrap_or(b'`');
    let n = src.bytes().take_while(|b| *b == ch).count();
    let mut l1 = a.line_at(range.end.saturating_sub(1).max(range.start)).max(l0);
    let closed = l1 > l0 && is_closing_fence(&text[a.lines[l1].start..a.lines[l1].end], ch, n);
    if !closed {
        // An unclosed fence swallows trailing blank lines up to the end of the document.
        let tail_blank = a.lines[l1 + 1..].iter().all(|l| text[l.start..l.end].trim().is_empty());
        if tail_blank {
            l1 = a.lines.len() - 1;
        }
        if opener_ok {
            a.unclosed.push(l0);
        }
    }
    let lang = info.split_whitespace().next().unwrap_or("").to_string();
    for li in l0..=l1 {
        let line = &mut a.lines[li];
        if li == l0 {
            if !opener_ok {
                continue;
            }
            line.lead = Some(Lead::Fence);
            line.code = Some(CodeLine { block: s0, role: CodeRole::Open { lang: lang.clone() } });
        } else if li == l1 && closed {
            line.lead = Some(Lead::Fence);
            line.code = Some(CodeLine { block: s0, role: CodeRole::Close });
        } else {
            line.code = Some(CodeLine { block: s0, role: CodeRole::Body });
        }
    }
}

/// An empty `- ` / `1. ` right under a paragraph is not a list item to
/// CommonMark (an empty item cannot interrupt a paragraph) but is one to the
/// user who just typed it.
fn fallback_items(text: &str, a: &mut Analysis) {
    for i in 1..a.lines.len() {
        let (prev_blank, l) = {
            let p = &a.lines[i - 1];
            (text[p.start..p.end].trim().is_empty(), &a.lines[i])
        };
        if prev_blank || l.lead.is_some() || l.code.is_some() || l.kind != LineKind::Normal {
            continue;
        }
        let s = &text[l.start..l.end];
        let indent = s.len() - s.trim_start_matches(' ').len();
        let Some(m) = item_marker(&s[indent..]) else { continue };
        if !s[indent + m.len..].trim().is_empty() || m.ordered.is_some_and(|n| n != 1) {
            continue;
        }
        let marker = l.start..l.end;
        // Depth: one deeper than the nearest list line above that is indented less.
        let mut depth = 0;
        for j in (0..i).rev() {
            let p = &a.lines[j];
            if text[p.start..p.end].trim().is_empty() {
                break;
            }
            let pindent = text[p.start..p.end].len() - text[p.start..p.end].trim_start_matches(' ').len();
            let pd = match &p.lead {
                Some(Lead::Bullet { depth, .. } | Lead::Ordered { depth, .. } | Lead::Task { depth, .. }) => Some(*depth),
                _ => None,
            };
            if let Some(pd) = pd
                && pindent < indent
            {
                depth = pd + 1;
                break;
            }
        }
        let lead = if m.ordered.is_some() {
            Lead::Ordered { marker, depth, number: 1 }
        } else {
            Lead::Bullet { marker, depth }
        };
        a.lines[i].lead = Some(lead);
    }
}

/// Distributes spans to lines (never into code lines) and indexes delimiters.
fn distribute(a: &mut Analysis, spans: Vec<Span>) {
    for s in spans {
        if s.range.is_empty() {
            continue;
        }
        let l0 = a.line_at(s.range.start);
        let l1 = a.line_at(s.range.end.saturating_sub(1).max(s.range.start)).max(l0);
        for line in &mut a.lines[l0..=l1] {
            if line.code.is_some() {
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
        if a.lines[a.line_at(inl.full.start)].code.is_some() {
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

// ---- fences & Enter ----------------------------------------------------------

/// `(indent, fence)` when `line` looks like an opening fence (```lang or ~~~).
pub fn fence_opener(line: &str) -> Option<(&str, &str)> {
    let t = line.trim_start_matches(' ');
    let indent = &line[..line.len() - t.len()];
    let ch = t.chars().next().filter(|c| matches!(c, '`' | '~'))?;
    let n = t.chars().take_while(|c| *c == ch).count();
    let info = &t[n..];
    (n >= 3 && (ch == '~' || !info.contains('`'))).then_some((indent, &t[..n]))
}

/// What pressing Enter at a list/quote line should do.
#[derive(Debug, PartialEq, Eq)]
pub enum Continuation {
    /// Insert a newline followed by this prefix.
    Continue(String),
    /// The item was empty: clear the line instead.
    Clear,
}

pub fn continuation(text: &str, line: &Line) -> Option<Continuation> {
    let lead = line.lead.as_ref()?;
    let (marker, next) = match lead {
        Lead::Bullet { marker, .. } => (marker, text[marker.clone()].to_string()),
        Lead::Ordered { marker, number, .. } => {
            let m = &text[marker.clone()];
            let d = m.find(|c: char| c.is_ascii_digit())?;
            let e = d + m[d..].bytes().take_while(u8::is_ascii_digit).count();
            (marker, format!("{}{}{}", &m[..d], number + 1, &m[e..]))
        }
        Lead::Task { marker, bracket, .. } => (marker, format!("{}[ ] ", &text[marker.start..bracket.start])),
        Lead::Quote { marker } => {
            let m = &text[marker.clone()];
            (marker, if m.ends_with(' ') { m.to_string() } else { format!("{m} ") })
        }
        _ => return None,
    };
    if text[marker.end..line.end].trim().is_empty() {
        Some(Continuation::Clear)
    } else {
        Some(Continuation::Continue(next))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn an(t: &str) -> Analysis {
        analyze(t, 0)
    }

    fn lead(t: &str, line: usize) -> Option<Lead> {
        an(t).lines[line].lead.clone()
    }

    #[test]
    fn headings() {
        let a = an("# One\n\n### Three\n#### Four\n#nospace\nplain");
        assert_eq!(a.lines[0].kind, LineKind::Heading(1));
        assert_eq!(a.lines[0].lead, Some(Lead::Heading { marker: 0..2 }));
        assert_eq!(a.lines[2].kind, LineKind::Heading(3));
        assert_eq!(a.lines[3].kind, LineKind::Heading(4));
        assert_eq!(a.lines[4].kind, LineKind::Normal);
        assert_eq!(a.lines[5].kind, LineKind::Normal);
    }

    #[test]
    fn marker_alone_is_not_complete() {
        for t in ["#", "-", "1.", ">", "##"] {
            assert_eq!(an(t).lines[0].lead, None, "{t:?}");
        }
        assert!(an("# ").lines[0].lead.is_some());
        assert!(an("- ").lines[0].lead.is_some());
    }

    #[test]
    fn bullets_numbers_and_tasks() {
        let t = "- a\n* b\n\n1. c\n2. d\n\n- [ ] e\n- [x] f\n- [X] g";
        assert_eq!(lead(t, 0), Some(Lead::Bullet { marker: 0..2, depth: 0 }));
        assert!(matches!(lead(t, 1), Some(Lead::Bullet { .. })));
        assert!(matches!(lead(t, 3), Some(Lead::Ordered { number: 1, .. })));
        assert!(matches!(lead(t, 4), Some(Lead::Ordered { number: 2, .. })));
        match lead(t, 6) {
            Some(Lead::Task { marker, checked, bracket, .. }) => {
                assert!(!checked);
                assert_eq!(&t[marker], "- [ ] ");
                assert_eq!(&t[bracket], "[ ]");
            }
            o => panic!("{o:?}"),
        }
        assert!(matches!(lead(t, 7), Some(Lead::Task { checked: true, .. })));
        assert!(matches!(lead(t, 8), Some(Lead::Task { checked: true, .. })));
    }

    #[test]
    fn empty_task_is_a_task() {
        assert!(matches!(lead("- [ ] ", 0), Some(Lead::Task { .. })));
        assert!(matches!(lead("- [ ]", 0), Some(Lead::Bullet { .. })));
    }

    #[test]
    fn ordered_numbers_are_renumbered_and_nested() {
        let t = "1. a\n1. b\n   - x\n   - y\n1. c";
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
    fn empty_item_under_paragraph_is_still_an_item() {
        let t = "text\n- ";
        assert!(matches!(lead(t, 1), Some(Lead::Bullet { depth: 0, .. })));
        assert_eq!(an(t).lines[0].kind, LineKind::Normal);
        let t = "- a\n  - ";
        assert!(matches!(lead(t, 1), Some(Lead::Bullet { depth: 1, .. })));
        assert!(matches!(lead("text\n1. ", 1), Some(Lead::Ordered { .. })));
    }

    #[test]
    fn setext_is_not_a_heading_but_dashes_are_a_divider() {
        let a = an("Title\n---\nafter");
        assert_eq!(a.lines[0].kind, LineKind::Normal);
        assert_eq!(a.lines[1].kind, LineKind::Rule);
        let a = an("Title\n===");
        assert_eq!(a.lines[0].kind, LineKind::Normal);
        assert_eq!(a.lines[1].kind, LineKind::Normal);
        let a = an("Title\n--");
        assert_eq!(a.lines[1].kind, LineKind::Normal);
    }

    #[test]
    fn quotes_rules_and_fences() {
        let t = "> q\n\n---\n\n```\ncode\n```\n\nafter";
        let a = an(t);
        assert_eq!(a.lines[0].kind, LineKind::Quote);
        assert_eq!(a.lines[0].lead, Some(Lead::Quote { marker: 0..2 }));
        assert_eq!(a.lines[2].kind, LineKind::Rule);
        assert_eq!(a.lines[4].lead, Some(Lead::Fence));
        assert_eq!(a.lines[5].kind, LineKind::Code);
        assert_eq!(a.lines[5].lead, None);
        assert_eq!(a.lines[6].lead, Some(Lead::Fence));
        assert_eq!(a.lines[8].kind, LineKind::Normal);
    }

    #[test]
    fn code_roles_and_language() {
        let a = an("```rust\nfn x() {}\n```");
        assert_eq!(a.lines[0].code.as_ref().unwrap().role, CodeRole::Open { lang: "rust".into() });
        assert_eq!(a.lines[1].code.as_ref().unwrap().role, CodeRole::Body);
        assert_eq!(a.lines[2].code.as_ref().unwrap().role, CodeRole::Close);
        assert_eq!(a.block_lines(1), Some((0, Some(2))));
        assert!(a.unclosed.is_empty());
    }

    #[test]
    fn unclosed_fence_runs_to_end_per_commonmark() {
        let t = "```\na\n\nb\n";
        let a = analyze(t, t.len());
        assert!(a.lines.iter().all(|l| l.kind == LineKind::Code));
        assert_eq!(a.unclosed, vec![0]);
    }

    #[test]
    fn unclosed_fence_on_the_caret_line_is_plain_text() {
        let t = "```\nrest\n**x**";
        assert_eq!(analyze(t, 3).lines[1].kind, LineKind::Normal);
        assert!(analyze(t, 3).lines[0].code.is_none());
        assert_eq!(analyze(t, 6).lines[1].kind, LineKind::Code);
        // a closed block never reacts to the caret
        let t = "```\nrest\n```";
        assert_eq!(analyze(t, 3).lines[1].kind, LineKind::Code);
    }

    #[test]
    fn adjacent_blocks_have_distinct_ids() {
        let a = an("```\na\n```\n```\nb\n```");
        let ids: Vec<_> = a.lines.iter().map(|l| l.code.as_ref().unwrap().block).collect();
        assert_eq!(ids, vec![0, 0, 0, 10, 10, 10]);
    }

    #[test]
    fn fence_in_list_item() {
        let a = an("- a\n  ```\n  c\n  ```\nafter");
        assert!(matches!(a.lines[0].lead, Some(Lead::Bullet { .. })));
        assert_eq!(a.lines[1].code.as_ref().unwrap().role, CodeRole::Open { lang: String::new() });
        assert_eq!(a.lines[2].kind, LineKind::Code);
        assert_eq!(a.lines[3].code.as_ref().unwrap().role, CodeRole::Close);
        assert_eq!(a.lines[4].kind, LineKind::Normal);
    }

    #[test]
    fn indented_code_is_plain() {
        let a = an("para\n\n    not code");
        assert_eq!(a.lines[2].kind, LineKind::Normal);
        assert!(a.lines[2].code.is_none());
    }

    #[test]
    fn inline_spans_and_delims() {
        let t = "a **b** *i* `c` ~~s~~";
        let a = an(t);
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
        let a = an(t);
        assert_eq!(a.inlines[0].url.as_deref(), Some("http://x.y"));
        assert_eq!(&t[a.inlines[0].open.clone()], "[");
        assert_eq!(&t[a.inlines[0].close.clone()], "](http://x.y)");
    }

    #[test]
    fn no_inline_in_code_blocks() {
        let a = an("```\n**x**\n```");
        assert!(a.inlines.is_empty());
        assert!(a.lines[1].spans.is_empty());
    }

    #[test]
    fn line_at_and_coverage() {
        let a = an("a\n\nbc\n");
        assert_eq!(a.lines.len(), 4);
        assert_eq!((a.lines[2].start, a.lines[2].end), (3, 5));
        assert_eq!(a.line_at(4), 2);
        assert_eq!(a.line_at(6), 3);
    }

    #[test]
    fn fence_opener_detection() {
        assert_eq!(fence_opener("```rust"), Some(("", "```")));
        assert_eq!(fence_opener("  ~~~"), Some(("  ", "~~~")));
        assert_eq!(fence_opener("````"), Some(("", "````")));
        assert_eq!(fence_opener("``"), None);
        assert_eq!(fence_opener("```a`b"), None);
        assert_eq!(fence_opener("x```"), None);
    }

    fn cont(t: &str) -> Option<Continuation> {
        let a = an(t);
        continuation(t, a.lines.last().unwrap())
    }

    #[test]
    fn enter_in_lists() {
        assert_eq!(cont("- foo"), Some(Continuation::Continue("- ".into())));
        assert_eq!(cont("- a\n  * foo"), Some(Continuation::Continue("  * ".into())));
        assert_eq!(cont("1. a\n2. foo"), Some(Continuation::Continue("3. ".into())));
        assert_eq!(cont("> hi"), Some(Continuation::Continue("> ".into())));
        assert_eq!(cont("- [x] done"), Some(Continuation::Continue("- [ ] ".into())));
        assert_eq!(cont("- [ ] todo"), Some(Continuation::Continue("- [ ] ".into())));
        assert_eq!(cont("- "), Some(Continuation::Clear));
        assert_eq!(cont("- [ ] "), Some(Continuation::Clear));
        assert_eq!(cont("1. "), Some(Continuation::Clear));
        assert_eq!(cont("plain"), None);
        assert_eq!(cont("-no"), None);
    }
}
