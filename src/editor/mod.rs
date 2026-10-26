//! The markdown editing surface: a GPUI shell around the headless `Model`.
//!
//! The model owns the buffer, selection, decorations and every key behaviour
//! (see `model.rs`); this file adds layout-dependent navigation (wrapped lines,
//! hit-testing), mouse input, IME plumbing and rendering.

pub mod buffer;
mod element;
pub mod layout;
pub mod model;
#[cfg(test)]
mod tests;

use crate::actions::*;
use crate::search;
use crate::settings::Settings;
use crate::slash;
use crate::theme::Theme;
use buffer::Buffer;
use element::EditorElement;
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, EntityInputHandler, FocusHandle, Focusable,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    ParentElement, Pixels, Point, Render, ScrollHandle, StatefulInteractiveElement, Styled,
    UTF16Selection, Window, anchored, deferred, div, point, prelude::*, px,
};
use layout::Layout;
use model::Model;
use std::cell::RefCell;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;

pub const COLUMN_WIDTH: f32 = 680.0;

#[derive(Default)]
pub struct Find {
    pub query: String,
    pub matches: Vec<Range<usize>>,
    pub current: Option<usize>,
}

#[derive(PartialEq)]
pub(crate) struct LayoutKey {
    revision: u64,
    views_gen: u64,
    width: u32,
    font_size: u32,
    font: crate::settings::FontChoice,
    dark: bool,
}

type LayoutCache = Rc<RefCell<Option<(LayoutKey, Rc<Layout>)>>>;

pub struct Editor {
    pub focus_handle: FocusHandle,
    pub m: Model,
    goal_x: Option<Pixels>,
    pub scroll: ScrollHandle,
    layout_cache: LayoutCache,
    origin: Point<Pixels>,
    selecting: bool,
    pub(crate) need_scroll: bool,
    pub path: Option<PathBuf>,
    saved_revision: u64,
    pub find: Find,
    pub placeholder: &'static str,
}

impl Focusable for Editor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Editor {
    pub fn new(text: String, path: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let m = Model::new(text);
        let saved_revision = m.buffer.revision();
        Self {
            focus_handle: cx.focus_handle(),
            m,
            goal_x: None,
            scroll: ScrollHandle::new(),
            layout_cache: Rc::new(RefCell::new(None)),
            origin: point(px(0.), px(0.)),
            selecting: false,
            need_scroll: false,
            path,
            saved_revision,
            find: Find::default(),
            placeholder: crate::placeholders::random(),
        }
    }

    // ---- document state -----------------------------------------------------

    pub fn text(&self) -> &str {
        self.m.text()
    }

    pub fn buffer(&self) -> &Buffer {
        &self.m.buffer
    }

    /// The visible text without markdown markers or decoration (for counts).
    pub fn plain_text(&self) -> String {
        self.m.plain_text()
    }

    pub fn is_dirty(&self) -> bool {
        self.m.buffer.revision() != self.saved_revision
    }

    pub fn file_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".into())
    }

    /// Replaces the document (used by New and Open).
    pub fn load(&mut self, text: String, path: Option<PathBuf>, cx: &mut Context<Self>) {
        self.m.load(text);
        self.saved_revision = self.m.buffer.revision();
        self.path = path;
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.placeholder = crate::placeholders::random();
        self.changed(cx);
    }

    /// Records a successful save of `revision` (the revision that was written).
    pub fn mark_saved(&mut self, path: PathBuf, revision: u64, cx: &mut Context<Self>) {
        self.path = Some(path);
        self.saved_revision = revision;
        cx.notify();
    }

    pub fn selected_text(&self) -> Option<&str> {
        self.m.selected_text()
    }

    /// Call after any model mutation: refreshes find matches and schedules a repaint.
    fn changed(&mut self, cx: &mut Context<Self>) {
        if !self.find.query.is_empty() {
            self.refresh_matches();
        }
        self.goal_x = None;
        self.need_scroll = true;
        cx.notify();
    }

    // ---- find ---------------------------------------------------------------

    pub fn set_find_query(&mut self, query: &str, cx: &mut Context<Self>) {
        self.find.query = query.to_string();
        self.refresh_matches();
        self.find.current = search::next_index(&self.find.matches, self.m.selection.start);
        self.reveal_current_match();
        cx.notify();
    }

    pub fn clear_find(&mut self, cx: &mut Context<Self>) {
        self.find = Find::default();
        cx.notify();
    }

    pub fn find_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let m = &self.find.matches;
        if m.is_empty() {
            return;
        }
        let sel = &self.m.selection;
        let idx = match (self.find.current, forward) {
            (Some(i), true) => Some((i + 1) % m.len()),
            (Some(i), false) => Some((i + m.len() - 1) % m.len()),
            (None, true) => search::next_index(m, sel.end),
            (None, false) => search::prev_index(m, sel.start),
        };
        self.find.current = idx;
        self.reveal_current_match();
        cx.notify();
    }

    fn refresh_matches(&mut self) {
        self.find.matches = search::find_all(self.m.text(), &self.find.query);
        if self.find.current.is_some_and(|i| i >= self.find.matches.len()) {
            self.find.current = None;
        }
    }

    fn reveal_current_match(&mut self) {
        if let Some(r) = self.find.current.and_then(|i| self.find.matches.get(i)).cloned() {
            self.m.selection = r;
            self.m.reversed = false;
            self.need_scroll = true;
        }
    }

    // ---- selection ----------------------------------------------------------

    fn cursor(&self) -> usize {
        self.m.cursor()
    }

    fn set_cursor(&mut self, offset: usize, select: bool, cx: &mut Context<Self>) {
        self.m.set_cursor(offset, select);
        self.need_scroll = true;
        cx.notify();
    }

    fn horizontal(&mut self, target: usize, select: bool, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.set_cursor(target, select, cx);
    }

    fn vertical(&mut self, dir: i32, select: bool, cx: &mut Context<Self>) {
        let Some(layout) = self.current_layout() else { return };
        let Some((pt, lh)) = layout.point_for_offset(self.cursor()) else { return };
        let goal = *self.goal_x.get_or_insert(pt.x);
        let y = if dir > 0 { pt.y + lh + px(1.) } else { pt.y - px(1.) };
        let target = if y < px(0.) {
            0
        } else if y >= layout.height {
            self.m.buffer.len()
        } else {
            layout.offset_for_point(point(goal, y))
        };
        self.set_cursor(target, select, cx);
    }

    fn current_layout(&self) -> Option<Rc<Layout>> {
        self.layout_cache.borrow().as_ref().map(|(_, l)| l.clone())
    }

    /// Cached layout for the given width; rebuilt when text, style or conceal state changes.
    pub(crate) fn layout_for(&mut self, width: Pixels, window: &mut Window, cx: &mut App) -> Rc<Layout> {
        let settings = cx.global::<Settings>().clone();
        let theme = *cx.global::<Theme>();
        let key = LayoutKey {
            revision: self.m.buffer.revision(),
            views_gen: self.m.views_gen,
            width: f32::from(width).to_bits(),
            font_size: settings.font_size.to_bits(),
            font: settings.font,
            dark: theme.dark,
        };
        if let Some((k, l)) = self.layout_cache.borrow().as_ref()
            && *k == key
        {
            return l.clone();
        }
        let layout = Rc::new(layout::build(window, self.m.views.clone(), self.m.buffer.len(), &settings, &theme, width));
        *self.layout_cache.borrow_mut() = Some((key, layout.clone()));
        layout
    }

    // ---- action handlers ----------------------------------------------------

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.m.move_left(false);
        self.moved(cx);
    }
    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        self.goal_x = None;
        self.m.move_right(false);
        self.moved(cx);
    }
    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        if self.m.slash.is_some() {
            self.m.slash_move(-1);
            cx.notify();
            return;
        }
        self.vertical(-1, false, cx);
    }
    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        if self.m.slash.is_some() {
            self.m.slash_move(1);
            cx.notify();
            return;
        }
        self.vertical(1, false, cx);
    }
    fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.m.prev_word_pos(self.cursor()), false, cx);
    }
    fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.m.next_word_pos(self.cursor()), false, cx);
    }
    fn line_start(&mut self, _: &LineStart, _: &mut Window, cx: &mut Context<Self>) {
        let t = self.visual_line_edge(true);
        self.horizontal(t, false, cx);
    }
    fn line_end(&mut self, _: &LineEnd, _: &mut Window, cx: &mut Context<Self>) {
        let t = self.visual_line_edge(false);
        self.horizontal(t, false, cx);
    }
    fn doc_start(&mut self, _: &DocStart, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(0, false, cx);
    }
    fn doc_end(&mut self, _: &DocEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.m.buffer.len(), false, cx);
    }
    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.m.prev_pos(self.cursor()), true, cx);
    }
    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.m.next_pos(self.cursor()), true, cx);
    }
    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(-1, true, cx);
    }
    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(1, true, cx);
    }
    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.m.prev_word_pos(self.cursor()), true, cx);
    }
    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.m.next_word_pos(self.cursor()), true, cx);
    }
    fn select_line_start(&mut self, _: &SelectLineStart, _: &mut Window, cx: &mut Context<Self>) {
        let t = self.visual_line_edge(true);
        self.horizontal(t, true, cx);
    }
    fn select_line_end(&mut self, _: &SelectLineEnd, _: &mut Window, cx: &mut Context<Self>) {
        let t = self.visual_line_edge(false);
        self.horizontal(t, true, cx);
    }
    fn select_doc_start(&mut self, _: &SelectDocStart, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(0, true, cx);
    }
    fn select_doc_end(&mut self, _: &SelectDocEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.m.buffer.len(), true, cx);
    }
    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.m.select_all();
        cx.notify();
    }

    /// Cursor-only change: repaint and keep the caret visible.
    fn moved(&mut self, cx: &mut Context<Self>) {
        self.need_scroll = true;
        cx.notify();
    }

    /// Start/end of the visual (wrapped) row containing the caret.
    fn visual_line_edge(&self, start: bool) -> usize {
        let cursor = self.cursor();
        if let Some(layout) = self.current_layout()
            && let Some((pt, lh)) = layout.point_for_offset(cursor)
        {
            let x = if start { px(0.) } else { layout.width + px(1000.) };
            return layout.offset_for_point(point(x, pt.y + lh / 2.));
        }
        self.m.logical_line_edge(start)
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        self.m.backspace();
        self.changed(cx);
    }
    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        self.m.delete();
        self.changed(cx);
    }
    fn delete_word_left(&mut self, _: &DeleteWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.m.delete_word_left();
        self.changed(cx);
    }
    fn delete_to_line_start(&mut self, _: &DeleteToLineStart, _: &mut Window, cx: &mut Context<Self>) {
        self.m.delete_to_line_start();
        self.changed(cx);
    }
    fn newline(&mut self, _: &Newline, _: &mut Window, cx: &mut Context<Self>) {
        self.m.enter();
        self.changed(cx);
    }
    fn tab(&mut self, _: &Tab, _: &mut Window, cx: &mut Context<Self>) {
        self.m.tab();
        self.changed(cx);
    }
    fn backtab(&mut self, _: &Backtab, _: &mut Window, cx: &mut Context<Self>) {
        self.m.backtab();
        self.changed(cx);
    }
    fn toggle_check(&mut self, _: &ToggleCheck, _: &mut Window, cx: &mut Context<Self>) {
        self.m.toggle_check();
        self.changed(cx);
    }

    fn cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        if self.m.slash.take().is_some() {
            cx.notify();
        } else {
            cx.propagate();
        }
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(t) = self.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(t.to_string()));
        }
    }
    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(t) = self.m.cut() {
            cx.write_to_clipboard(ClipboardItem::new_string(t));
            self.changed(cx);
        }
    }
    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(t) = cx.read_from_clipboard().and_then(|i| i.text()) {
            self.m.insert(&t);
            self.changed(cx);
        }
    }
    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        self.m.undo();
        self.changed(cx);
    }
    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        self.m.redo();
        self.changed(cx);
    }

    // ---- mouse --------------------------------------------------------------

    fn offset_at(&self, p: Point<Pixels>) -> usize {
        match self.current_layout() {
            Some(l) => l.offset_for_point(p - self.origin),
            None => 0,
        }
    }

    /// The line whose checkbox contains `p`, if any.
    fn checkbox_at(&self, p: Point<Pixels>) -> Option<usize> {
        let layout = self.current_layout()?;
        let rel = p - self.origin;
        let i = layout.lines.partition_point(|l| l.y + l.height <= rel.y);
        let r = layout.checkbox_rect(i)?;
        let hit = Bounds::new(r.origin - point(px(4.), px(4.)), r.size + gpui::size(px(8.), px(8.)));
        hit.contains(&rel).then_some(i)
    }

    fn link_at(&self, off: usize) -> Option<String> {
        self.m
            .analysis
            .inlines
            .iter()
            .find(|i| i.url.is_some() && i.full.start <= off && off <= i.full.end)
            .and_then(|i| i.url.clone())
    }

    fn mouse_down(&mut self, e: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        if let Some(li) = self.checkbox_at(e.position) {
            self.m.toggle_task(li);
            self.changed(cx);
            return;
        }
        let off = self.offset_at(e.position);
        if e.modifiers.platform
            && let Some(url) = self.link_at(off)
        {
            cx.open_url(&url);
            return;
        }
        self.selecting = true;
        self.goal_x = None;
        match e.click_count {
            2 => {
                self.m.selection = self.m.buffer.word_range(off);
                self.m.reversed = false;
                cx.notify();
            }
            n if n >= 3 => {
                let end = (self.m.buffer.line_end(off) + 1).min(self.m.buffer.len());
                self.m.selection = self.m.buffer.line_start(off)..end;
                self.m.reversed = false;
                cx.notify();
            }
            _ => self.set_cursor(off, e.modifiers.shift, cx),
        }
        self.m.slash = None;
        self.need_scroll = false;
    }
    fn mouse_move(&mut self, e: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.selecting && e.dragging() {
            let off = self.offset_at(e.position);
            self.set_cursor(off, true, cx);
            self.need_scroll = false;
        }
    }
    fn mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }

    // ---- utf16 helpers (platform IME speaks UTF-16) --------------------------

    fn utf16_to_byte(&self, off: usize) -> usize {
        let (mut b, mut u) = (0, 0);
        for ch in self.text().chars() {
            if u >= off {
                break;
            }
            u += ch.len_utf16();
            b += ch.len_utf8();
        }
        b
    }
    fn to_utf16(&self, off: usize) -> usize {
        let (mut u, mut b) = (0, 0);
        for ch in self.text().chars() {
            if b >= off {
                break;
            }
            b += ch.len_utf8();
            u += ch.len_utf16();
        }
        u
    }
    fn range_to_utf16(&self, r: &Range<usize>) -> Range<usize> {
        self.to_utf16(r.start)..self.to_utf16(r.end)
    }
    fn range_from_utf16(&self, r: &Range<usize>) -> Range<usize> {
        self.utf16_to_byte(r.start)..self.utf16_to_byte(r.end)
    }
}

impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let r = self.range_from_utf16(&range_utf16);
        actual.replace(self.range_to_utf16(&r));
        Some(self.text()[r].to_string())
    }

    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        Some(UTF16Selection { range: self.range_to_utf16(&self.m.selection), reversed: self.m.reversed })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.m.marked.as_ref().map(|r| self.range_to_utf16(r))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.m.marked = None;
    }

    fn replace_text_in_range(&mut self, range_utf16: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        let range = range_utf16.as_ref().map(|r| self.range_from_utf16(r));
        self.m.replace_text(range, text);
        self.changed(cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        new_selected_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.m.marked.clone())
            .unwrap_or(self.m.selection.clone());
        self.m.insert_range(range.clone(), text);
        if !text.is_empty() {
            self.m.marked = Some(range.start..range.start + text.len());
        }
        if let Some(sel) = new_selected_utf16 {
            // Offsets are relative to the inserted text.
            let conv = |u: usize| {
                let (mut b, mut n) = (0, 0);
                for ch in text.chars() {
                    if n >= u {
                        break;
                    }
                    n += ch.len_utf16();
                    b += ch.len_utf8();
                }
                b
            };
            self.m.set_selection(range.start + conv(sel.start)..range.start + conv(sel.end));
        }
        self.changed(cx);
    }

    fn bounds_for_range(&mut self, range_utf16: Range<usize>, bounds: Bounds<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<Bounds<Pixels>> {
        let layout = self.current_layout()?;
        let r = self.range_from_utf16(&range_utf16);
        let (p, lh) = layout.point_for_offset(r.start)?;
        Some(Bounds::new(bounds.origin + p, gpui::size(px(2.), lh)))
    }

    fn character_index_for_point(&mut self, p: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        Some(self.to_utf16(self.offset_at(p)))
    }
}

impl Editor {
    fn slash_popup(&self, cx: &mut Context<Self>) -> Option<impl IntoElement + use<>> {
        let query = self.m.slash_query()?;
        let selected = self.m.slash.as_ref()?.selected;
        let layout = self.current_layout()?;
        let (p, lh) = layout.point_for_offset(self.cursor())?;
        let theme = *cx.global::<Theme>();
        let items = slash::filter(&query);
        let pos = self.origin + p + point(px(0.), lh + px(6.));
        let rows = items.iter().enumerate().map(|(i, item)| {
            div()
                .id(("slash", i))
                .flex()
                .justify_between()
                .gap(px(24.))
                .px(px(10.))
                .py(px(5.))
                .rounded(px(6.))
                .cursor_pointer()
                .when(i == selected, |d| d.bg(theme.hover))
                .hover(|d| d.bg(theme.hover))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _, cx| {
                        this.m.slash_pick(i);
                        this.changed(cx);
                        cx.stop_propagation();
                    }),
                )
                .child(item.label)
                .child(div().text_color(theme.muted).text_size(px(12.)).child(item.hint))
        });
        Some(deferred(
            anchored().position(pos).snap_to_window_with_margin(px(8.)).child(
                div()
                    .occlude()
                    .w(px(220.))
                    .p(px(5.))
                    .flex()
                    .flex_col()
                    .bg(theme.panel)
                    .border_1()
                    .border_color(theme.hairline)
                    .rounded(px(10.))
                    .shadow_lg()
                    .font_family(".SystemUIFont")
                    .text_size(px(13.))
                    .text_color(theme.fg)
                    .children(rows),
            ),
        ))
    }
}

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let popup = self.slash_popup(cx);
        div()
            .id("editor")
            .key_context("Editor")
            .track_focus(&self.focus_handle)
            .size_full()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::up))
            .on_action(cx.listener(Self::down))
            .on_action(cx.listener(Self::word_left))
            .on_action(cx.listener(Self::word_right))
            .on_action(cx.listener(Self::line_start))
            .on_action(cx.listener(Self::line_end))
            .on_action(cx.listener(Self::doc_start))
            .on_action(cx.listener(Self::doc_end))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_up))
            .on_action(cx.listener(Self::select_down))
            .on_action(cx.listener(Self::select_word_left))
            .on_action(cx.listener(Self::select_word_right))
            .on_action(cx.listener(Self::select_line_start))
            .on_action(cx.listener(Self::select_line_end))
            .on_action(cx.listener(Self::select_doc_start))
            .on_action(cx.listener(Self::select_doc_end))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::delete_word_left))
            .on_action(cx.listener(Self::delete_to_line_start))
            .on_action(cx.listener(Self::newline))
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::backtab))
            .on_action(cx.listener(Self::toggle_check))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::undo))
            .on_action(cx.listener(Self::redo))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::mouse_down))
            .on_mouse_move(cx.listener(Self::mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::mouse_up))
            .child(
                div().flex().justify_center().w_full().child(
                    div()
                        .w_full()
                        .max_w(px(COLUMN_WIDTH + 48.))
                        .px(px(24.))
                        .pt(px(72.))
                        // At least toolbar + fade band so the last line scrolls fully clear.
                        .pb(px(crate::toolbar::HEIGHT + crate::edge_blur::BOTTOM_EXTRA + 140.))
                        .child(EditorElement { editor: cx.entity() }),
                ),
            )
            .children(popup)
    }
}

impl Editor {
    pub(crate) fn set_origin(&mut self, origin: Point<Pixels>) {
        self.origin = origin;
    }
    pub(crate) fn caret(&self) -> usize {
        self.cursor()
    }
    pub(crate) fn selection(&self) -> Range<usize> {
        self.m.selection.clone()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.m.buffer.is_empty()
    }
}
