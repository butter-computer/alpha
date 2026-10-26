//! Text layout for the editor: one shaped (and possibly wrapped) line per
//! logical line, styled from the markdown analysis.

use crate::markdown::{self, Analysis, LineKind};
use crate::settings::{FontChoice, MONO_FAMILY, Settings};
use crate::theme::Theme;
use gpui::{
    Bounds, Font, FontStyle, FontWeight, Hsla, Pixels, Point, StrikethroughStyle, TextRun,
    UnderlineStyle, Window, WrappedLine, font, point, px, size, transparent_black,
};
use std::ops::Range;

pub struct LineLayout {
    pub start: usize,
    pub end: usize,
    pub kind: LineKind,
    /// Top of this line's block, relative to the element origin.
    pub y: Pixels,
    /// Extra space above the first text row (headings).
    pub pad_top: Pixels,
    pub height: Pixels,
    pub line_height: Pixels,
    pub wrapped: Option<WrappedLine>,
}

pub struct Layout {
    pub lines: Vec<LineLayout>,
    pub width: Pixels,
    pub height: Pixels,
    pub text_len: usize,
}

const HEADING_SCALE: [f32; 6] = [1.8, 1.45, 1.25, 1.12, 1.05, 1.0];

pub struct Metrics {
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub pad_top: Pixels,
}

pub fn metrics(kind: LineKind, base: f32, first: bool) -> Metrics {
    match kind {
        LineKind::Heading(n) => {
            let size = base * HEADING_SCALE[(n.clamp(1, 6) - 1) as usize];
            Metrics {
                font_size: px(size),
                line_height: px(size * 1.3),
                pad_top: if first { px(0.) } else { px(size * 0.45) },
            }
        }
        LineKind::Code => Metrics { font_size: px(base * 0.86), line_height: px(base * 1.5), pad_top: px(0.) },
        _ => Metrics { font_size: px(base), line_height: px(base * 1.6), pad_top: px(0.) },
    }
}

pub fn build(
    window: &mut Window,
    text: &str,
    analysis: &Analysis,
    settings: &Settings,
    theme: &Theme,
    active_line: usize,
    width: Pixels,
) -> Layout {
    let base_family = settings.font.family();
    let mut y = px(0.);
    let mut lines = Vec::with_capacity(analysis.lines.len());
    for (i, line) in analysis.lines.iter().enumerate() {
        let m = metrics(line.kind, settings.font_size, i == 0);
        let src = &text[line.start..line.end];
        let wrapped = if src.is_empty() {
            None
        } else {
            let runs = runs_for_line(line, base_family, theme, i == active_line, settings.font);
            window
                .text_system()
                .shape_text(src.to_string().into(), m.font_size, &runs, Some(width), None)
                .ok()
                .and_then(|mut v| v.pop())
        };
        let rows = wrapped.as_ref().map(|w| w.wrap_boundaries.len() + 1).unwrap_or(1);
        let height = m.pad_top + m.line_height * rows as f32;
        lines.push(LineLayout {
            start: line.start,
            end: line.end,
            kind: line.kind,
            y,
            pad_top: m.pad_top,
            height,
            line_height: m.line_height,
            wrapped,
        });
        y += height;
    }
    Layout { lines, width, height: y, text_len: text.len() }
}

fn runs_for_line(line: &markdown::Line, base_family: &'static str, theme: &Theme, active: bool, choice: FontChoice) -> Vec<TextRun> {
    let in_code = line.kind == LineKind::Code;
    line.segments()
        .into_iter()
        .map(|(range, flags)| {
            let mono = in_code || flags & markdown::CODE != 0;
            let family = if mono && choice != FontChoice::Mono { MONO_FAMILY } else { base_family };
            let mut f: Font = font(family);
            if flags & markdown::BOLD != 0 && !mono {
                f.weight = FontWeight::MEDIUM;
            }
            if flags & markdown::ITALIC != 0 {
                f.style = FontStyle::Italic;
            }
            let mut color: Hsla = match line.kind {
                LineKind::Quote => theme.fg.opacity(0.72),
                LineKind::Code => theme.fg.opacity(0.88),
                _ => theme.fg,
            };
            if flags & markdown::LINK != 0 {
                color = theme.accent;
            }
            if flags & markdown::MARKER != 0 {
                color = if active {
                    theme.marker
                } else if line.kind == LineKind::Rule {
                    transparent_black()
                } else {
                    theme.marker_faint
                };
            }
            TextRun {
                len: range.len(),
                font: f,
                color,
                background_color: (flags & markdown::CODE != 0 && !in_code).then_some(theme.code_bg),
                underline: (flags & markdown::LINK != 0).then(|| UnderlineStyle {
                    color: Some(theme.accent.opacity(0.5)),
                    thickness: px(1.),
                    wavy: false,
                }),
                strikethrough: (flags & markdown::STRIKE != 0).then(|| StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(theme.fg.opacity(0.6)),
                }),
            }
        })
        .collect()
}

impl Layout {
    pub fn line_index_for_offset(&self, offset: usize) -> usize {
        match self.lines.binary_search_by(|l| l.start.cmp(&offset)) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        }
    }

    /// Top-left of the caret for `offset` and the height of its row.
    pub fn point_for_offset(&self, offset: usize) -> Option<(Point<Pixels>, Pixels)> {
        let line = self.lines.get(self.line_index_for_offset(offset))?;
        let local = offset.clamp(line.start, line.end) - line.start;
        let p = match &line.wrapped {
            Some(w) => w.position_for_index(local, line.line_height)?,
            None => point(px(0.), px(0.)),
        };
        Some((point(p.x, line.y + line.pad_top + p.y), line.line_height))
    }

    /// Closest byte offset to a point relative to the element origin.
    pub fn offset_for_point(&self, p: Point<Pixels>) -> usize {
        if p.y < px(0.) {
            return 0;
        }
        let i = self.lines.partition_point(|l| l.y + l.height <= p.y);
        let Some(line) = self.lines.get(i) else { return self.text_len };
        let Some(w) = &line.wrapped else { return line.start };
        let local = point(p.x, (p.y - line.y - line.pad_top).max(px(0.)));
        let idx = match w.closest_index_for_position(local, line.line_height) {
            Ok(i) | Err(i) => i,
        };
        line.start + idx.min(line.end - line.start)
    }

    /// Index of the first line that is not entirely above `top`.
    pub fn first_visible(&self, top: Pixels) -> usize {
        self.lines.partition_point(|l| l.y + l.height < top)
    }

    /// Highlight rectangles (relative to the element origin) for an absolute byte range.
    pub fn rects_for_range(&self, line: &LineLayout, range: Range<usize>) -> Vec<Bounds<Pixels>> {
        let mut out = Vec::new();
        let r_start = range.start.max(line.start);
        let r_end = range.end.min(line.end);
        let includes_newline = range.end > line.end && range.start <= line.end;
        let nl_extra = px(6.);
        let Some(w) = &line.wrapped else {
            if includes_newline || (range.start <= line.start && range.end > line.start) {
                out.push(Bounds::new(point(px(0.), line.y + line.pad_top), size(nl_extra, line.line_height)));
            }
            return out;
        };
        let ul = &w.unwrapped_layout;
        let mut starts = vec![0usize];
        starts.extend(w.wrap_boundaries.iter().map(|b| ul.runs[b.run_ix].glyphs[b.glyph_ix].index));
        let len = line.end - line.start;
        let (a, b) = (r_start.saturating_sub(line.start), r_end.saturating_sub(line.start));
        for (row, &row_start) in starts.iter().enumerate() {
            let row_end = starts.get(row + 1).copied().unwrap_or(len);
            let last_row = row + 1 == starts.len();
            let (s, e) = (a.max(row_start), b.min(row_end));
            let (x0, mut x1);
            if s < e {
                x0 = ul.x_for_index(s) - ul.x_for_index(row_start);
                x1 = ul.x_for_index(e) - ul.x_for_index(row_start);
            } else if last_row && includes_newline {
                x0 = ul.x_for_index(len) - ul.x_for_index(row_start);
                x1 = x0;
            } else {
                continue;
            }
            if last_row && includes_newline {
                x1 += nl_extra;
            }
            out.push(Bounds::new(
                point(x0, line.y + line.pad_top + line.line_height * row as f32),
                size((x1 - x0).max(px(0.)), line.line_height),
            ));
        }
        out
    }
}
