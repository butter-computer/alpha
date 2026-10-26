//! The conceal/decoration layer.
//!
//! The buffer stays plain markdown. For every line this module computes a
//! *display string* in which syntax markers are removed (concealed) or replaced
//! by a decoration prefix (bullet glyph, number, checkbox placeholder), plus a
//! mapping between buffer offsets and display offsets. Layout shapes the display
//! string; cursor movement, hit-testing and selection go through the mapping so
//! concealed text behaves as if it did not exist.
//!
//! Conventions: a display offset that is ambiguous (it sits next to a concealed
//! range) maps to the buffer offset *after* the concealed range, and the caret
//! can never sit inside a line's decoration prefix.

use crate::markdown::{Analysis, CODE, CodeLine, DONE, Lead, Line, LineKind, PREFIX};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegKind {
    /// Text shown as-is.
    Visible,
    /// Buffer text with no display (zero width).
    Concealed,
    /// Buffer text replaced by decoration text.
    Prefix,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seg {
    pub buf: Range<usize>,
    pub disp: Range<usize>,
    pub kind: SegKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkbox {
    /// Placeholder range in the display string (one em space).
    pub disp: Range<usize>,
    pub checked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineView {
    pub start: usize,
    pub end: usize,
    /// Effective kind (a revealed line is a plain paragraph).
    pub kind: LineKind,
    pub display: String,
    pub segs: Vec<Seg>,
    /// (display byte length, style flags) runs covering `display`.
    pub runs: Vec<(usize, u8)>,
    /// The caret may not move left of this display offset.
    pub min_disp: usize,
    pub checkbox: Option<Checkbox>,
    /// Set for lines of a fenced code block.
    pub code: Option<CodeLine>,
    /// A fence line that is concealed (the caret is not on it).
    pub fence_hidden: bool,
    /// Display ranges of inline code (for the rounded background).
    pub code_spans: Vec<Range<usize>>,
}

impl LineView {
    /// Display offset (relative to the line) for a buffer offset.
    pub fn buf_to_disp(&self, buf: usize) -> usize {
        let buf = buf.clamp(self.start, self.end);
        let mut last = 0;
        for s in &self.segs {
            if buf <= s.buf.end {
                return match s.kind {
                    SegKind::Visible => s.disp.start + buf.saturating_sub(s.buf.start),
                    SegKind::Concealed => s.disp.start,
                    SegKind::Prefix => s.disp.end,
                };
            }
            last = s.disp.end;
        }
        last
    }

    /// Buffer offset for a display offset; ambiguity resolves to *after*
    /// concealed text, and never lands inside the prefix.
    pub fn disp_to_buf(&self, disp: usize) -> usize {
        let d = disp.min(self.display.len());
        let mut result = None;
        for s in &self.segs {
            match s.kind {
                SegKind::Visible => {
                    if s.disp.start <= d && d <= s.disp.end {
                        result = Some(s.buf.start + (d - s.disp.start));
                    }
                }
                SegKind::Concealed => {
                    if s.disp.start == d {
                        result = Some(s.buf.end);
                    }
                }
                SegKind::Prefix => {
                    if d <= s.disp.end {
                        result = Some(s.buf.end);
                    }
                }
            }
        }
        result.unwrap_or(self.end)
    }

    pub fn normalize(&self, buf: usize) -> usize {
        self.disp_to_buf(self.buf_to_disp(buf))
    }

    /// Buffer offset where editing content starts (after any marker).
    pub fn content_start(&self) -> usize {
        self.disp_to_buf(self.min_disp)
    }

    /// Visible text without decoration.
    pub fn plain(&self) -> &str {
        &self.display[self.min_disp.min(self.display.len())..]
    }
}

/// Which markers are currently shown raw. It names the exact marker range it
/// revealed, so it can be re-validated against any later text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reveal {
    /// The block marker of one line (a divider's marker is its whole line).
    Block { marker: Range<usize> },
    /// The delimiters of one inline span.
    Inline { open: Range<usize>, close: Range<usize> },
}

impl Reveal {
    /// Lines the reveal belongs to: a block marker's line, or the lines holding
    /// the opening and closing delimiters of an inline span.
    fn lines(&self, a: &Analysis) -> [usize; 2] {
        match self {
            Reveal::Block { marker } => [a.line_at(marker.start); 2],
            Reveal::Inline { open, close } => [a.line_at(open.start), a.line_at(close.start)],
        }
    }

    /// Keeps the ranges anchored after an edit (`removed` bytes at `at` replaced by `inserted`).
    /// Ranges the edit touches shrink or move, so they stop matching and the reveal clears.
    pub fn adjust(&mut self, at: usize, removed: usize, inserted: usize) {
        let shift = |p: usize| {
            if p <= at {
                p
            } else if p >= at + removed {
                p + inserted - removed
            } else {
                at + inserted
            }
        };
        let map = |r: &mut Range<usize>| *r = shift(r.start)..shift(r.end);
        match self {
            Reveal::Block { marker } => map(marker),
            Reveal::Inline { open, close } => {
                map(open);
                map(close);
            }
        }
    }

    /// Whether the reveal still applies: its marker range exists in the current
    /// analysis and the caret is on its line.
    pub fn is_valid(&self, a: &Analysis, caret: usize) -> bool {
        let lines = self.lines(a);
        if !lines.contains(&a.line_at(caret)) {
            return false;
        }
        match self {
            Reveal::Block { marker } => a.lines[lines[0]].marker().as_ref() == Some(marker),
            Reveal::Inline { open, close } => a.inlines.iter().any(|i| i.open == *open && i.close == *close),
        }
    }
}

/// Decides what Backspace does for a collapsed caret: `Some` reveals markers
/// (and deletes nothing); `None` means delete normally.
pub fn on_backspace(a: &Analysis, caret: usize, current: &Option<Reveal>) -> Option<Reveal> {
    let line = &a.lines[a.line_at(caret)];
    if let (Some(lead), Some(marker)) = (&line.lead, line.marker()) {
        let already = matches!(current, Some(Reveal::Block { marker: m }) if *m == marker);
        if !already && caret == lead.content_start(line) && marker.end > marker.start {
            return Some(Reveal::Block { marker });
        }
    }
    for inl in &a.inlines {
        let at_close = !inl.close.is_empty() && inl.close.end == caret;
        let at_open = !inl.open.is_empty() && inl.open.end == caret;
        let already = matches!(current, Some(Reveal::Inline { open, close }) if *open == inl.open && *close == inl.close);
        if (at_close || at_open) && !already {
            return Some(Reveal::Inline { open: inl.open.clone(), close: inl.close.clone() });
        }
    }
    None
}

const EM: &str = "\u{2003}";
const EN: &str = "\u{2002}";

fn bullet_glyph(depth: usize) -> &'static str {
    match depth {
        0 => "•",
        1 => "◦",
        _ => "▪",
    }
}

/// The whole decoration state as a pure function of the text, the caret and
/// the reveal state. A reveal that no longer applies is ignored.
pub fn build_views(text: &str, a: &Analysis, reveal: &Option<Reveal>, caret: usize) -> Vec<LineView> {
    let reveal = reveal.as_ref().filter(|r| r.is_valid(a, caret));
    let caret_line = a.line_at(caret.min(text.len()));
    a.lines.iter().enumerate().map(|(i, l)| build_line(text, a, l, reveal, i == caret_line)).collect()
}

fn build_line(text: &str, a: &Analysis, line: &Line, reveal: Option<&Reveal>, caret_here: bool) -> LineView {
    let block_revealed = matches!(reveal, Some(Reveal::Block { marker }) if line.marker().as_ref() == Some(marker));
    let revealed_inline = match reveal {
        Some(Reveal::Inline { open, close }) => a.inlines.iter().position(|i| i.open == *open && i.close == *close),
        _ => None,
    };
    // Fence lines show raw only while the caret is on them.
    let fence_shown = line.is_fence() && caret_here;
    let lead = if block_revealed || fence_shown { None } else { line.lead.clone() };
    let kind = if block_revealed { LineKind::Normal } else { line.kind };

    let mut view = LineView {
        start: line.start,
        end: line.end,
        kind,
        display: String::new(),
        segs: Vec::new(),
        runs: Vec::new(),
        min_disp: 0,
        checkbox: None,
        code: line.code.clone(),
        fence_hidden: false,
        code_spans: Vec::new(),
    };

    let mut pos = line.start;
    let mut done = 0u8;
    let prefix: Option<(String, Range<usize>)> = match &lead {
        Some(Lead::Fence) | Some(Lead::Rule) => {
            if line.end > line.start {
                view.segs.push(Seg { buf: line.start..line.end, disp: 0..0, kind: SegKind::Concealed });
            }
            view.fence_hidden = matches!(lead, Some(Lead::Fence));
            return view;
        }
        Some(Lead::Heading { marker }) | Some(Lead::Quote { marker }) => {
            if !marker.is_empty() {
                view.segs.push(Seg { buf: marker.clone(), disp: 0..0, kind: SegKind::Concealed });
            }
            pos = marker.end;
            None
        }
        Some(Lead::Bullet { marker, depth }) => {
            pos = marker.end;
            Some((format!("{}{}{EN}", EM.repeat(*depth), bullet_glyph(*depth)), marker.clone()))
        }
        Some(Lead::Ordered { marker, depth, number }) => {
            pos = marker.end;
            Some((format!("{}{number}.{EN}", EM.repeat(*depth)), marker.clone()))
        }
        Some(Lead::Task { marker, depth, checked, .. }) => {
            pos = marker.end;
            let indent = EM.repeat(*depth);
            let p = format!("{indent}{EM}");
            view.checkbox = Some(Checkbox {
                disp: indent.len()..p.len(),
                checked: *checked,
            });
            if *checked {
                done = DONE;
            }
            Some((p, marker.clone()))
        }
        None => None,
    };
    if let Some((p, marker)) = prefix {
        view.segs.push(Seg { buf: line.start..marker.end, disp: 0..p.len(), kind: SegKind::Prefix });
        view.runs.push((p.len(), PREFIX));
        view.min_disp = p.len();
        view.display.push_str(&p);
    }

    // Concealed inline delimiters on this line.
    let concealed: Vec<Range<usize>> = line
        .delims
        .iter()
        .filter(|(_, idx)| Some(*idx) != revealed_inline)
        .map(|(r, _)| r.clone())
        .filter(|r| r.end > pos)
        .collect();

    let mut cuts = vec![pos, line.end];
    for s in &line.spans {
        cuts.push(s.range.start);
        cuts.push(s.range.end);
    }
    for r in &concealed {
        cuts.push(r.start);
        cuts.push(r.end);
    }
    cuts.retain(|c| *c >= pos && *c <= line.end);
    cuts.sort_unstable();
    cuts.dedup();
    for w in cuts.windows(2) {
        let (x, y) = (w[0], w[1]);
        if concealed.iter().any(|c| c.start <= x && y <= c.end) {
            let d = view.display.len();
            view.segs.push(Seg { buf: x..y, disp: d..d, kind: SegKind::Concealed });
            continue;
        }
        let style = if kind == LineKind::Code {
            0
        } else {
            line.spans.iter().filter(|s| s.range.start <= x && y <= s.range.end).fold(0, |f, s| f | s.flags)
        };
        let flags = style | done;
        let d = view.display.len();
        view.display.push_str(&text[x..y]);
        view.segs.push(Seg { buf: x..y, disp: d..d + (y - x), kind: SegKind::Visible });
        match view.runs.last_mut() {
            Some((len, f)) if *f == flags => *len += y - x,
            _ => view.runs.push((y - x, flags)),
        }
    }
    let mut at = 0;
    for &(len, flags) in &view.runs {
        if flags & CODE != 0 && kind != LineKind::Code {
            match view.code_spans.last_mut() {
                Some(r) if r.end == at => r.end = at + len,
                _ => view.code_spans.push(at..at + len),
            }
        }
        at += len;
    }
    view
}

/// All visible content, one line per buffer line (no markers or decoration).
pub fn plain_text(views: &[LineView]) -> String {
    let mut out = String::new();
    for (i, v) in views.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(v.plain());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::analyze;

    fn views_at(t: &str, caret: usize) -> Vec<LineView> {
        build_views(t, &analyze(t, caret), &None, caret)
    }

    fn views(t: &str) -> Vec<LineView> {
        views_at(t, t.len())
    }

    #[test]
    fn heading_marker_is_hidden() {
        let v = &views("# Title")[0];
        assert_eq!(v.display, "Title");
        assert_eq!(v.kind, LineKind::Heading(1));
        // buffer offsets inside the marker snap to the content start
        assert_eq!(v.buf_to_disp(0), 0);
        assert_eq!(v.buf_to_disp(1), 0);
        assert_eq!(v.buf_to_disp(2), 0);
        assert_eq!(v.buf_to_disp(4), 2);
        assert_eq!(v.disp_to_buf(0), 2);
        assert_eq!(v.disp_to_buf(2), 4);
        assert_eq!(v.normalize(0), 2);
        assert_eq!(v.content_start(), 2);
    }

    #[test]
    fn bold_delimiters_are_hidden_and_map_round_trip() {
        let t = "a **b** c";
        let v = &views(t)[0];
        assert_eq!(v.display, "a b c");
        // display offset before 'b' maps to inside the bold span (after the opener)
        assert_eq!(v.disp_to_buf(2), 4);
        // display offset after 'b' maps after the closer
        assert_eq!(v.disp_to_buf(3), 7);
        assert_eq!(v.buf_to_disp(4), 2);
        assert_eq!(v.buf_to_disp(5), 3);
        assert_eq!(v.buf_to_disp(7), 3);
        assert_eq!(v.buf_to_disp(8), 4);
        for d in 0..=v.display.len() {
            assert_eq!(v.buf_to_disp(v.disp_to_buf(d)), d, "d={d}");
        }
    }

    #[test]
    fn bullet_is_replaced_by_glyph_and_caret_stays_after_it() {
        let v = &views("- item")[0];
        assert_eq!(v.display, "•\u{2002}item");
        assert_eq!(v.min_disp, "•\u{2002}".len());
        assert_eq!(v.disp_to_buf(0), 2);
        assert_eq!(v.disp_to_buf(v.min_disp), 2);
        assert_eq!(v.buf_to_disp(0), v.min_disp);
        assert_eq!(v.buf_to_disp(2), v.min_disp);
        assert_eq!(v.plain(), "item");
    }

    #[test]
    fn nested_bullets_use_other_glyphs() {
        let vs = views("- a\n  - b\n    - c");
        assert!(vs[1].display.starts_with("\u{2003}◦"));
        assert!(vs[2].display.starts_with("\u{2003}\u{2003}▪"));
    }

    #[test]
    fn numbered_lists_renumber() {
        let vs = views("1. a\n1. b\n1. c");
        assert!(vs[0].display.starts_with("1."));
        assert!(vs[1].display.starts_with("2."));
        assert!(vs[2].display.starts_with("3."));
    }

    #[test]
    fn task_has_checkbox_and_done_style() {
        let v = &views("- [x] done")[0];
        let cb = v.checkbox.as_ref().unwrap();
        assert!(cb.checked);
        assert_eq!(&v.display[cb.disp.clone()], "\u{2003}");
        assert_eq!(v.plain(), "done");
        assert!(v.runs.iter().any(|(_, f)| f & DONE != 0));
    }

    #[test]
    fn quote_marker_hidden_rule_line_empty() {
        let vs = views("> q\n\n---");
        assert_eq!(vs[0].display, "q");
        assert_eq!(vs[2].display, "");
        assert_eq!(vs[2].disp_to_buf(0), vs[2].end);
    }

    #[test]
    fn fences_are_hidden_unless_the_caret_is_on_them() {
        let t = "```rust\ncode\n```";
        let away = views_at(t, 9);
        assert_eq!(away[0].display, "");
        assert!(away[0].fence_hidden && away[2].fence_hidden);
        assert_eq!(away[1].display, "code");
        assert_eq!(away[1].kind, LineKind::Code);
        let on_open = views_at(t, 3);
        assert_eq!(on_open[0].display, "```rust");
        assert!(!on_open[0].fence_hidden);
        assert!(on_open[2].fence_hidden);
        let on_close = views_at(t, t.len());
        assert_eq!(on_close[2].display, "```");
    }

    #[test]
    fn code_body_gets_no_inline_styling() {
        let v = &views_at("```\n**x** `y`\n```", 6)[1];
        assert_eq!(v.display, "**x** `y`");
        assert!(v.runs.iter().all(|(_, f)| *f == 0));
        assert!(v.code_spans.is_empty());
    }

    #[test]
    fn inline_code_reports_display_ranges() {
        let v = &views("a `bc` d")[0];
        assert_eq!(v.display, "a bc d");
        assert_eq!(v.code_spans, vec![2..4]);
    }

    #[test]
    fn link_hides_brackets_and_url() {
        let v = &views("see [text](http://x.y) ok")[0];
        assert_eq!(v.display, "see text ok");
    }

    #[test]
    fn block_reveal_shows_raw_plain_paragraph() {
        let t = "# Title";
        let a = analyze(t, 2);
        let r = Some(Reveal::Block { marker: 0..2 });
        let v = &build_views(t, &a, &r, 2)[0];
        assert_eq!(v.display, "# Title");
        assert_eq!(v.kind, LineKind::Normal);
        // a reveal for a marker that is not there is ignored
        let r = Some(Reveal::Block { marker: 0..1 });
        assert_eq!(build_views(t, &a, &r, 2)[0].display, "Title");
    }

    #[test]
    fn inline_reveal_only_that_span() {
        let t = "**a** and *b*";
        let a = analyze(t, 5);
        let r = Some(Reveal::Inline { open: 0..2, close: 3..5 });
        let v = &build_views(t, &a, &r, 5)[0];
        assert_eq!(v.display, "**a** and b");
    }

    #[test]
    fn plain_text_ignores_markers() {
        let t = "# Hi there\n\n- a **b**\n\n```\nc\n```\n\n---";
        let vs = views_at(t, 0);
        assert_eq!(plain_text(&vs), "Hi there\n\na b\n\n\nc\n\n\n");
    }

    #[test]
    fn backspace_after_block_marker_reveals_instead_of_deleting() {
        let t = "# Title";
        let a = analyze(t, 2);
        let m = Reveal::Block { marker: 0..2 };
        assert_eq!(on_backspace(&a, 2, &None), Some(m.clone()));
        // once revealed, backspace edits normally
        assert_eq!(on_backspace(&a, 2, &Some(m)), None);
        // elsewhere in the line: normal
        assert_eq!(on_backspace(&a, 4, &None), None);
    }

    #[test]
    fn backspace_after_bullet_and_checklist_marker_reveals() {
        for t in ["- x", "1. x", "- [ ] x", "> x"] {
            let a = analyze(t, 0);
            let cs = a.lines[0].lead.as_ref().unwrap().content_start(&a.lines[0]);
            assert_eq!(on_backspace(&a, cs, &None), Some(Reveal::Block { marker: 0..cs }), "{t}");
        }
    }

    #[test]
    fn backspace_reveals_empty_marker_only_line() {
        let a = analyze("# ", 2);
        assert_eq!(on_backspace(&a, 2, &None), Some(Reveal::Block { marker: 0..2 }));
    }

    #[test]
    fn backspace_on_a_divider_reveals_its_dashes() {
        let a = analyze("---", 3);
        assert_eq!(on_backspace(&a, 3, &None), Some(Reveal::Block { marker: 0..3 }));
    }

    #[test]
    fn backspace_never_reveals_fences() {
        let t = "```\n\n```";
        let a = analyze(t, 3);
        assert_eq!(on_backspace(&a, 3, &None), None);
    }

    #[test]
    fn backspace_after_closing_delimiter_reveals_that_span() {
        let t = "**a** *b*";
        let a = analyze(t, 5);
        let first = Reveal::Inline { open: 0..2, close: 3..5 };
        assert_eq!(on_backspace(&a, 5, &None), Some(first.clone()));
        assert_eq!(on_backspace(&a, 5, &Some(first.clone())), None);
        // the other span is independent
        assert_eq!(on_backspace(&a, 9, &Some(first)), Some(Reveal::Inline { open: 6..7, close: 8..9 }));
        // caret right after an opener also reveals
        assert_eq!(on_backspace(&a, 2, &None), Some(Reveal::Inline { open: 0..2, close: 3..5 }));
    }

    #[test]
    fn reveal_validity_follows_the_caret_and_the_marker() {
        let t = "# a\nplain";
        let a = analyze(t, 0);
        let r = Reveal::Block { marker: 0..2 };
        assert!(r.is_valid(&a, 2));
        assert!(r.is_valid(&a, 3));
        assert!(!r.is_valid(&a, 4));
        assert!(!Reveal::Block { marker: 0..1 }.is_valid(&a, 2));
        let t = "**a** b";
        let a = analyze(t, 0);
        let i = Reveal::Inline { open: 0..2, close: 3..5 };
        assert!(i.is_valid(&a, 6));
        assert!(!Reveal::Inline { open: 0..2, close: 3..6 }.is_valid(&a, 6));
    }

    #[test]
    fn reveal_adjusts_for_edits() {
        let mut r = Reveal::Inline { open: 4..6, close: 8..10 };
        r.adjust(0, 0, 2); // insert before
        assert_eq!(r, Reveal::Inline { open: 6..8, close: 10..12 });
        r.adjust(9, 1, 0); // delete between
        assert_eq!(r, Reveal::Inline { open: 6..8, close: 9..11 });
        let mut b = Reveal::Block { marker: 10..12 };
        b.adjust(2, 0, 3);
        assert_eq!(b, Reveal::Block { marker: 13..15 });
        // an edit inside the marker changes its range
        b.adjust(14, 1, 0);
        assert_eq!(b, Reveal::Block { marker: 13..14 });
    }
}
