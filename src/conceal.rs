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

use crate::markdown::{Analysis, DONE, Lead, Line, LineKind, PREFIX};
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

#[derive(Clone, Debug)]
pub struct Seg {
    pub buf: Range<usize>,
    pub disp: Range<usize>,
    pub kind: SegKind,
}

#[derive(Clone, Debug)]
pub struct Checkbox {
    /// Placeholder range in the display string (one em space).
    pub disp: Range<usize>,
    pub checked: bool,
}

#[derive(Clone, Debug)]
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

/// Which markers are currently shown raw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reveal {
    /// A block marker on the line starting at `line_start`.
    Block { line_start: usize },
    /// The delimiters of one inline span.
    Inline { full: Range<usize> },
}

impl Reveal {
    /// Keeps the reveal anchored after an edit (`removed` bytes at `at` replaced by `inserted`).
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
        match self {
            Reveal::Block { line_start } => *line_start = shift(*line_start),
            Reveal::Inline { full } => {
                let (s, e) = (shift(full.start), shift(full.end));
                *full = s..e.max(s);
            }
        }
    }

    /// Whether the reveal still applies for a caret at `caret`.
    pub fn is_valid(&self, a: &Analysis, caret: usize) -> bool {
        match self {
            Reveal::Block { line_start } => a.lines[a.line_at(caret)].start == *line_start,
            Reveal::Inline { full } => full.start <= caret && caret <= full.end,
        }
    }
}

/// Decides what Backspace does for a collapsed caret: `Some` reveals markers
/// (and deletes nothing); `None` means delete normally.
pub fn on_backspace(a: &Analysis, caret: usize, current: &Option<Reveal>) -> Option<Reveal> {
    let line = &a.lines[a.line_at(caret)];
    if let Some(lead) = &line.lead {
        let already = matches!(current, Some(Reveal::Block { line_start }) if *line_start == line.start);
        if !already && caret == lead.content_start(line) && (caret > line.start || line.end > line.start) {
            return Some(Reveal::Block { line_start: line.start });
        }
    }
    for inl in &a.inlines {
        let at_close = !inl.close.is_empty() && inl.close.end == caret;
        let at_open = !inl.open.is_empty() && inl.open.end == caret;
        if (at_close || at_open) && !matches!(current, Some(Reveal::Inline { full }) if *full == inl.full) {
            return Some(Reveal::Inline { full: inl.full.clone() });
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

pub fn build_views(text: &str, a: &Analysis, reveal: &Option<Reveal>) -> Vec<LineView> {
    a.lines.iter().map(|l| build_line(text, a, l, reveal)).collect()
}

fn build_line(text: &str, a: &Analysis, line: &Line, reveal: &Option<Reveal>) -> LineView {
    let block_revealed = matches!(reveal, Some(Reveal::Block { line_start }) if *line_start == line.start);
    let revealed_inline = match reveal {
        Some(Reveal::Inline { full }) => a.inlines.iter().position(|i| i.full == *full),
        _ => None,
    };
    let lead = if block_revealed { None } else { line.lead.clone() };
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
    };

    let mut pos = line.start;
    let mut done = 0u8;
    let prefix: Option<(String, Range<usize>)> = match &lead {
        Some(Lead::Fence) | Some(Lead::Rule) => {
            if line.end > line.start {
                view.segs.push(Seg { buf: line.start..line.end, disp: 0..0, kind: SegKind::Concealed });
            }
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

    fn views(t: &str) -> Vec<LineView> {
        build_views(t, &analyze(t), &None)
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
        let vs = views("- a\n    - b\n        - c");
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
        let t = "- [x] done";
        let a = analyze(t);
        let v = &build_views(t, &a, &None)[0];
        let cb = v.checkbox.as_ref().unwrap();
        assert!(cb.checked);
        assert_eq!(&v.display[cb.disp.clone()], "\u{2003}");
        assert_eq!(v.plain(), "done");
        assert!(v.runs.iter().any(|(_, f)| f & DONE != 0));
    }

    #[test]
    fn quote_marker_hidden_fence_and_rule_lines_empty() {
        let vs = views("> q\n```\nx\n```\n---");
        assert_eq!(vs[0].display, "q");
        assert_eq!(vs[1].display, "");
        assert_eq!(vs[2].display, "x");
        assert_eq!(vs[3].display, "");
        assert_eq!(vs[4].display, "");
        assert_eq!(vs[1].disp_to_buf(0), vs[1].end);
    }

    #[test]
    fn link_hides_brackets_and_url() {
        let v = &views("see [text](http://x.y) ok")[0];
        assert_eq!(v.display, "see text ok");
    }

    #[test]
    fn block_reveal_shows_raw_plain_paragraph() {
        let t = "# Title";
        let a = analyze(t);
        let v = &build_views(t, &a, &Some(Reveal::Block { line_start: 0 }))[0];
        assert_eq!(v.display, "# Title");
        assert_eq!(v.kind, LineKind::Normal);
    }

    #[test]
    fn inline_reveal_only_that_span() {
        let t = "**a** and *b*";
        let a = analyze(t);
        let full = a.inlines[0].full.clone();
        let v = &build_views(t, &a, &Some(Reveal::Inline { full }))[0];
        assert_eq!(v.display, "**a** and b");
    }

    #[test]
    fn plain_text_ignores_markers() {
        let vs = views("# Hi there\n- a **b**\n```\nc\n```\n---");
        assert_eq!(plain_text(&vs), "Hi there\na b\n\nc\n\n");
    }

    #[test]
    fn backspace_after_block_marker_reveals_instead_of_deleting() {
        let t = "# Title";
        let a = analyze(t);
        assert_eq!(on_backspace(&a, 2, &None), Some(Reveal::Block { line_start: 0 }));
        // once revealed, backspace edits normally
        assert_eq!(on_backspace(&a, 2, &Some(Reveal::Block { line_start: 0 })), None);
        // elsewhere in the line: normal
        assert_eq!(on_backspace(&a, 4, &None), None);
    }

    #[test]
    fn backspace_after_bullet_and_checklist_marker_reveals() {
        for t in ["- x", "1. x", "- [ ] x", "> x"] {
            let a = analyze(t);
            let cs = a.lines[0].lead.as_ref().unwrap().content_start(&a.lines[0]);
            assert_eq!(on_backspace(&a, cs, &None), Some(Reveal::Block { line_start: 0 }), "{t}");
        }
    }

    #[test]
    fn backspace_reveals_empty_marker_only_line() {
        let a = analyze("# ");
        assert_eq!(on_backspace(&a, 2, &None), Some(Reveal::Block { line_start: 0 }));
    }

    #[test]
    fn backspace_after_closing_delimiter_reveals_that_span() {
        let t = "**a** *b*";
        let a = analyze(t);
        assert_eq!(on_backspace(&a, 5, &None), Some(Reveal::Inline { full: 0..5 }));
        assert_eq!(on_backspace(&a, 5, &Some(Reveal::Inline { full: 0..5 })), None);
        // the other span is independent
        assert_eq!(on_backspace(&a, 9, &Some(Reveal::Inline { full: 0..5 })), Some(Reveal::Inline { full: 6..9 }));
        // caret right after an opener also reveals
        assert_eq!(on_backspace(&a, 2, &None), Some(Reveal::Inline { full: 0..5 }));
    }

    #[test]
    fn reveal_validity_follows_the_caret() {
        let t = "# a\nplain";
        let a = analyze(t);
        let r = Reveal::Block { line_start: 0 };
        assert!(r.is_valid(&a, 2));
        assert!(r.is_valid(&a, 3));
        assert!(!r.is_valid(&a, 4));
        let i = Reveal::Inline { full: 0..5 };
        assert!(i.is_valid(&a, 5));
        assert!(!i.is_valid(&a, 6));
    }

    #[test]
    fn reveal_adjusts_for_edits() {
        let mut r = Reveal::Inline { full: 4..9 };
        r.adjust(6, 0, 1); // insert inside
        assert_eq!(r, Reveal::Inline { full: 4..10 });
        r.adjust(0, 0, 2); // insert before
        assert_eq!(r, Reveal::Inline { full: 6..12 });
        r.adjust(7, 1, 0); // delete inside
        assert_eq!(r, Reveal::Inline { full: 6..11 });
        let mut b = Reveal::Block { line_start: 10 };
        b.adjust(2, 0, 3);
        assert_eq!(b, Reveal::Block { line_start: 13 });
    }
}
