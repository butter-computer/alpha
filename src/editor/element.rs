//! The custom GPUI element that lays out and paints the editor text.

use super::Editor;
use crate::theme::Theme;
use gpui::{
    App, AvailableSpace, Bounds, Element, ElementId, ElementInputHandler, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, Style, TextAlign, TextRun, Window, fill, point,
    px, relative, size,
};
use std::rc::Rc;

use super::layout::{Layout, metrics};
use crate::markdown::LineKind;
use crate::settings::Settings;

pub struct EditorElement {
    pub editor: Entity<Editor>,
}

impl IntoElement for EditorElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for EditorElement {
    type RequestLayoutState = ();
    type PrepaintState = Rc<Layout>;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        _cx: &mut App,
    ) -> (LayoutId, ()) {
        let editor = self.editor.clone();
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        let id = window.request_measured_layout(style, move |known, available, window, cx| {
            let width = known.width.unwrap_or(match available.width {
                AvailableSpace::Definite(w) => w,
                _ => px(super::COLUMN_WIDTH),
            });
            let layout = editor.update(cx, |ed, cx| ed.layout_for(width, window, cx));
            size(width, layout.height)
        });
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> Rc<Layout> {
        let layout = self
            .editor
            .update(cx, |ed, cx| ed.layout_for(bounds.size.width, window, cx));
        // Scroll the caret into view after edits/navigation.
        let (need, caret, scroll) = {
            let ed = self.editor.read(cx);
            (ed.need_scroll, ed.caret(), ed.scroll.clone())
        };
        if need {
            let viewport = scroll.bounds();
            if viewport.size.height > px(0.) {
                if let Some((p, lh)) = layout.point_for_offset(caret) {
                    let top = bounds.origin.y + p.y;
                    let bottom = top + lh;
                    // Keep the caret clear of the blur bands (top band / toolbar + fade ramp).
                    let top_margin = px(crate::edge_blur::TOP + 16.);
                    let bottom_margin = px(crate::toolbar::HEIGHT + crate::edge_blur::BOTTOM_EXTRA + 16.);
                    let mut off = scroll.offset();
                    if top < viewport.top() + top_margin {
                        off.y += viewport.top() + top_margin - top;
                    } else if bottom > viewport.bottom() - bottom_margin {
                        off.y -= bottom - (viewport.bottom() - bottom_margin);
                    }
                    if off != scroll.offset() {
                        scroll.set_offset(off);
                        window.refresh();
                    }
                }
                self.editor.update(cx, |ed, _| ed.need_scroll = false);
            }
        }
        layout
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        layout: &mut Rc<Layout>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let theme = *cx.global::<Theme>();
        let settings = cx.global::<Settings>().clone();
        let (focus, selection, caret, marked, empty, matches, current, placeholder) = {
            let ed = self.editor.read(cx);
            (
                ed.focus_handle.clone(),
                ed.selection(),
                ed.caret(),
                ed.marked(),
                ed.buffer.is_empty(),
                ed.find.matches.clone(),
                ed.find.current,
                ed.placeholder,
            )
        };
        let _ = marked;
        self.editor.update(cx, |ed, _| ed.set_origin(bounds.origin));
        window.handle_input(&focus, ElementInputHandler::new(bounds, self.editor.clone()), cx);

        let origin = bounds.origin;
        let visible = window.content_mask().bounds;
        let top = visible.top() - origin.y;
        let bottom = visible.bottom() - origin.y;
        let first = layout.first_visible(top);
        let mut last = first;
        while last < layout.lines.len() && layout.lines[last].y <= bottom {
            last += 1;
        }
        let lines = &layout.lines[first..last];

        // Block backgrounds: code blocks, quote bars, rules.
        let mut i = 0;
        while i < lines.len() {
            let l = &lines[i];
            match l.kind {
                LineKind::Code => {
                    let mut j = i;
                    while j + 1 < lines.len() && lines[j + 1].kind == LineKind::Code {
                        j += 1;
                    }
                    let y0 = l.y;
                    let y1 = lines[j].y + lines[j].height;
                    let mut q = fill(
                        Bounds::new(
                            point(origin.x - px(12.), origin.y + y0 - px(2.)),
                            size(bounds.size.width + px(24.), y1 - y0 + px(4.)),
                        ),
                        theme.code_bg,
                    );
                    q.corner_radii = px(6.).into();
                    window.paint_quad(q);
                    i = j;
                }
                LineKind::Quote => {
                    window.paint_quad(fill(
                        Bounds::new(point(origin.x - px(14.), origin.y + l.y), size(px(2.5), l.height)),
                        theme.quote_bar,
                    ));
                }
                LineKind::Rule => {
                    window.paint_quad(fill(
                        Bounds::new(
                            point(origin.x, origin.y + l.y + l.pad_top + l.line_height / 2.),
                            size(bounds.size.width, px(1.)),
                        ),
                        theme.quote_bar,
                    ));
                }
                _ => {}
            }
            i += 1;
        }

        // Find highlights.
        if !matches.is_empty() {
            let lo = lines.first().map(|l| l.start).unwrap_or(0);
            let hi = lines.last().map(|l| l.end).unwrap_or(0);
            let from = matches.partition_point(|m| m.end < lo);
            for (idx, m) in matches.iter().enumerate().skip(from) {
                if m.start > hi {
                    break;
                }
                let color = if Some(idx) == current { theme.find_current } else { theme.find };
                let li = layout.line_index_for_offset(m.start);
                for lj in li..layout.lines.len() {
                    if layout.lines[lj].start > m.end {
                        break;
                    }
                    for r in layout.rects_for_range(lj, m.clone()) {
                        let mut q = fill(Bounds::new(origin + r.origin, r.size), color);
                        q.corner_radii = px(2.).into();
                        window.paint_quad(q);
                    }
                }
            }
        }

        // Selection.
        if !selection.is_empty() {
            for li in first..last {
                let line = &layout.lines[li];
                if line.end < selection.start || line.start > selection.end {
                    continue;
                }
                for r in layout.rects_for_range(li, selection.clone()) {
                    window.paint_quad(fill(Bounds::new(origin + r.origin, r.size), theme.selection));
                }
            }
        }

        // Text.
        for line in lines {
            if let Some(w) = &line.wrapped {
                let o = point(origin.x, origin.y + line.y + line.pad_top);
                let _ = w.paint_background(o, line.line_height, TextAlign::Left, None, window, cx);
                let _ = w.paint(o, line.line_height, TextAlign::Left, None, window, cx);
            }
        }

        // Checkboxes.
        for li in first..last {
            let Some(cb) = layout.views[li].checkbox.as_ref() else { continue };
            let Some(r) = layout.checkbox_rect(li) else { continue };
            let b = Bounds::new(origin + r.origin, r.size);
            if cb.checked {
                let mut q = fill(b, theme.accent);
                q.corner_radii = px(3.).into();
                window.paint_quad(q);
                let text: gpui::SharedString = "✓".into();
                let run = TextRun {
                    len: text.len(),
                    font: gpui::font(settings.font.family()),
                    color: theme.bg,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let shaped = window.text_system().shape_line(text, r.size.height * 0.85, &[run], None);
                let w = shaped.width;
                let _ = shaped.paint(
                    point(b.origin.x + (b.size.width - w) / 2., b.origin.y + r.size.height * 0.04),
                    r.size.height,
                    window,
                    cx,
                );
            } else {
                window.paint_quad(gpui::quad(
                    b,
                    px(3.),
                    gpui::transparent_black(),
                    px(1.5),
                    theme.muted,
                    gpui::BorderStyle::Solid,
                ));
            }
        }

        // Placeholder.
        if empty {
            let m = metrics(LineKind::Normal, settings.font_size, true);
            let f = gpui::font(settings.font.family());
            let text: gpui::SharedString = placeholder.into();
            let run = TextRun {
                len: text.len(),
                font: f,
                color: theme.muted,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped = window.text_system().shape_line(text, m.font_size, &[run], None);
            let _ = shaped.paint(origin, m.line_height, window, cx);
        }

        // Caret.
        if selection.is_empty()
            && focus.is_focused(window)
            && let Some((p, lh)) = layout.point_for_offset(caret)
        {
            window.paint_quad(fill(Bounds::new(origin + p, size(px(2.), lh)), theme.accent));
        }
    }
}
