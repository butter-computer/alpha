//! Text layout for the editor: one shaped (and possibly wrapped) line per
//! logical line, built from the concealed *display* text of each line.

use crate::conceal::LineView;
use crate::markdown::{self, CodeRole, LineKind};
use crate::settings::{FontChoice, MONO_FAMILY, Settings};
use crate::theme::Theme;
use gpui::{
    Bounds, Font, FontStyle, FontWeight, Hsla, Pixels, Point, StrikethroughStyle, TextRun,
    UnderlineStyle, Window, WrappedLine, font, point, px, size,
};
use std::ops::Range;
use std::rc::Rc;

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
    pub font_size: Pixels,
    /// `None` when the display text is empty.
    pub wrapped: Option<WrappedLine>,
}

/// Outer margin of a code panel, and its inner padding.
pub const PANEL_MARGIN: f32 = 6.;
pub const PANEL_PAD: f32 = 12.;
/// Extra room under a concealed opening fence for the language label.
pub const PANEL_LABEL: f32 = 12.;

/// Height of a concealed fence row: the panel's margin plus its padding (and
/// label room), so the fence takes no more space than the panel's own padding.
fn hidden_fence_height(view: &LineView) -> f32 {
    match view.code.as_ref().map(|c| &c.role) {
        Some(CodeRole::Open { lang }) if !lang.is_empty() => PANEL_MARGIN + PANEL_PAD + PANEL_LABEL,
        _ => PANEL_MARGIN + PANEL_PAD,
    }
}

pub struct Layout {
    pub lines: Vec<LineLayout>,
    pub views: Rc<Vec<LineView>>,
    pub width: Pixels,
    pub height: Pixels,
    pub text_len: usize,
}

/// Heading sizes as multiples of the body size (levels 3+ share one size).
const HEADING_SCALE: [f32; 3] = [1.8, 1.45, 1.25];

pub struct Metrics {
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub pad_top: Pixels,
}

pub fn metrics(kind: LineKind, base: f32, first: bool) -> Metrics {
    match kind {
        LineKind::Heading(n) => {
            let size = base * HEADING_SCALE[(n.clamp(1, 3) - 1) as usize];
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

pub fn build(window: &mut Window, views: Rc<Vec<LineView>>, text_len: usize, settings: &Settings, theme: &Theme, width: Pixels) -> Layout {
    let base_family = settings.font.family();
    let mut y = px(0.);
    let mut lines = Vec::with_capacity(views.len());
    for (i, view) in views.iter().enumerate() {
        let mut m = metrics(view.kind, settings.font_size, i == 0);
        if view.fence_hidden {
            let h = px(hidden_fence_height(view));
            m = Metrics { font_size: m.font_size, line_height: h, pad_top: px(0.) };
        }
        let wrapped = if view.display.is_empty() {
            None
        } else {
            let runs = runs_for_view(view, base_family, theme, settings.font);
            window
                .text_system()
                .shape_text(view.display.clone().into(), m.font_size, &runs, Some(width), None)
                .ok()
                .and_then(|mut v| v.pop())
        };
        let rows = wrapped.as_ref().map(|w| w.wrap_boundaries.len() + 1).unwrap_or(1);
        let height = m.pad_top + m.line_height * rows as f32;
        lines.push(LineLayout {
            start: view.start,
            end: view.end,
            kind: view.kind,
            y,
            pad_top: m.pad_top,
            height,
            line_height: m.line_height,
            font_size: m.font_size,
            wrapped,
        });
        y += height;
    }
    Layout { lines, views, width, height: y, text_len }
}

fn runs_for_view(view: &LineView, base_family: &'static str, theme: &Theme, choice: FontChoice) -> Vec<TextRun> {
    let in_code = view.kind == LineKind::Code;
    view.runs
        .iter()
        .map(|&(len, flags)| {
            let mono = (in_code || flags & markdown::CODE != 0) && flags & markdown::PREFIX == 0;
            let family = if mono && choice != FontChoice::Mono { MONO_FAMILY } else { base_family };
            let mut f: Font = font(family);
            // Strong spans use SemiBold (never Bold); headings are regular weight.
            if flags & markdown::BOLD != 0 && !mono {
                f.weight = FontWeight::SEMIBOLD;
            }
            if flags & markdown::ITALIC != 0 {
                f.style = FontStyle::Italic;
            }
            let mut color: Hsla = match view.kind {
                LineKind::Quote => theme.fg.opacity(0.72),
                LineKind::Code => theme.fg.opacity(0.88),
                _ => theme.fg,
            };
            if flags & markdown::LINK != 0 {
                color = theme.accent;
            }
            if flags & markdown::MARKER != 0 {
                color = theme.marker;
            }
            if flags & markdown::PREFIX != 0 {
                color = theme.fg.opacity(0.55);
            }
            let done = flags & markdown::DONE != 0;
            if done {
                color = theme.muted;
            }
            TextRun {
                len,
                font: f,
                color,
                background_color: None,
                underline: (flags & markdown::LINK != 0).then(|| UnderlineStyle {
                    color: Some(theme.accent.opacity(0.5)),
                    thickness: px(1.),
                    wavy: false,
                }),
                strikethrough: (flags & markdown::STRIKE != 0 || done).then(|| StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(if done { theme.muted } else { theme.fg.opacity(0.6) }),
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

    /// Top-left of the caret for buffer `offset` and the height of its row.
    pub fn point_for_offset(&self, offset: usize) -> Option<(Point<Pixels>, Pixels)> {
        let i = self.line_index_for_offset(offset);
        let line = self.lines.get(i)?;
        let disp = self.views[i].buf_to_disp(offset);
        let p = match &line.wrapped {
            Some(w) => w.position_for_index(disp, line.line_height)?,
            None => point(px(0.), px(0.)),
        };
        Some((point(p.x, line.y + line.pad_top + p.y), line.line_height))
    }

    /// Closest buffer offset (never inside concealed text) to a point relative to the element origin.
    pub fn offset_for_point(&self, p: Point<Pixels>) -> usize {
        if p.y < px(0.) {
            return 0;
        }
        let i = self.lines.partition_point(|l| l.y + l.height <= p.y);
        let Some(line) = self.lines.get(i) else { return self.text_len };
        let view = &self.views[i];
        let Some(w) = &line.wrapped else { return view.disp_to_buf(0) };
        let local = point(p.x, (p.y - line.y - line.pad_top).max(px(0.)));
        let idx = match w.closest_index_for_position(local, line.line_height) {
            Ok(i) | Err(i) => i,
        };
        view.disp_to_buf(idx.min(view.display.len()))
    }

    /// Index of the first line that is not entirely above `top`.
    pub fn first_visible(&self, top: Pixels) -> usize {
        self.lines.partition_point(|l| l.y + l.height < top)
    }

    /// Checkbox square (relative to the element origin) for line `i`, if it has one.
    pub fn checkbox_rect(&self, i: usize) -> Option<Bounds<Pixels>> {
        let line = self.lines.get(i)?;
        let cb = self.views[i].checkbox.as_ref()?;
        let w = line.wrapped.as_ref()?;
        let a = w.position_for_index(cb.disp.start, line.line_height)?;
        let b = w.position_for_index(cb.disp.end, line.line_height)?;
        let box_size = line.font_size * 0.72;
        let x = a.x + (b.x - a.x - box_size) * 0.25;
        let y = line.y + line.pad_top + (line.line_height - box_size) / 2.;
        Some(Bounds::new(point(x, y), size(box_size, box_size)))
    }

    /// Highlight rectangles (relative to the element origin) for a buffer range on line `i`.
    pub fn rects_for_range(&self, i: usize, range: Range<usize>) -> Vec<Bounds<Pixels>> {
        let line = &self.lines[i];
        let view = &self.views[i];
        let includes_newline = range.end > line.end && range.start <= line.end;
        let a = view.buf_to_disp(range.start.max(line.start));
        let b = view.buf_to_disp(range.end.min(line.end));
        self.rects_for_disp(i, a..b, includes_newline)
    }

    /// Rectangles (one per wrapped row) for a display range on line `i`.
    pub fn rects_for_disp(&self, i: usize, disp: Range<usize>, includes_newline: bool) -> Vec<Bounds<Pixels>> {
        let line = &self.lines[i];
        let view = &self.views[i];
        let mut out = Vec::new();
        let nl_extra = px(6.);
        let Some(w) = &line.wrapped else {
            if includes_newline {
                out.push(Bounds::new(point(px(0.), line.y + line.pad_top), size(nl_extra, line.line_height)));
            }
            return out;
        };
        let ul = &w.unwrapped_layout;
        let mut starts = vec![0usize];
        starts.extend(w.wrap_boundaries.iter().map(|b| ul.runs[b.run_ix].glyphs[b.glyph_ix].index));
        let len = view.display.len();
        let (a, b) = (disp.start, disp.end);
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
