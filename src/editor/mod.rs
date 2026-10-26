//! The markdown editing surface: buffer, selection, input handling, layout.

pub mod buffer;
mod element;
pub mod layout;

use crate::actions::*;
use crate::markdown::{self, Analysis, Continuation};
use crate::search;
use crate::settings::Settings;
use crate::theme::Theme;
use buffer::Buffer;
use element::EditorElement;
use gpui::{
    App, Bounds, ClipboardItem, Context, CursorStyle, EntityInputHandler, FocusHandle,
    Focusable, InteractiveElement, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, ParentElement, Pixels, Point, Render, ScrollHandle, StatefulInteractiveElement,
    Styled, UTF16Selection, Window, div, point, px,
};
use layout::Layout;
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
    width: u32,
    font_size: u32,
    font: crate::settings::FontChoice,
    dark: bool,
    active_line: usize,
}

type LayoutCache = Rc<RefCell<Option<(LayoutKey, Rc<Layout>)>>>;

pub struct Editor {
    pub focus_handle: FocusHandle,
    pub buffer: Buffer,
    selection: Range<usize>,
    reversed: bool,
    marked: Option<Range<usize>>,
    pub analysis: Analysis,
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
        let buffer = Buffer::new(text);
        let saved_revision = buffer.revision();
        let analysis = markdown::analyze(buffer.text());
        Self {
            focus_handle: cx.focus_handle(),
            buffer,
            selection: 0..0,
            reversed: false,
            marked: None,
            analysis,
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
        self.buffer.text()
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.revision() != self.saved_revision
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
        self.buffer.reset(text);
        self.saved_revision = self.buffer.revision();
        self.path = path;
        self.selection = 0..0;
        self.reversed = false;
        self.marked = None;
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.placeholder = crate::placeholders::random();
        self.after_edit(cx);
    }

    /// Records a successful save of `revision` (the revision that was written).
    pub fn mark_saved(&mut self, path: PathBuf, revision: u64, cx: &mut Context<Self>) {
        self.path = Some(path);
        self.saved_revision = revision;
        cx.notify();
    }

    pub fn selected_text(&self) -> Option<&str> {
        (!self.selection.is_empty()).then(|| &self.text()[self.selection.clone()])
    }

    // ---- find ---------------------------------------------------------------

    pub fn set_find_query(&mut self, query: &str, cx: &mut Context<Self>) {
        self.find.query = query.to_string();
        self.refresh_matches();
        self.find.current = search::next_index(&self.find.matches, self.selection.start);
        self.reveal_current_match();
        cx.notify();
    }

    pub fn clear_find(&mut self, cx: &mut Context<Self>) {
        self.find = Find::default();
        cx.notify();
    }

    pub fn find_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        let m = &self.find.matches;
        let idx = match (self.find.current, forward) {
            (Some(i), true) => Some((i + 1) % m.len()),
            (Some(i), false) => Some((i + m.len() - 1) % m.len()),
            (None, true) => search::next_index(m, self.selection.end),
            (None, false) => search::prev_index(m, self.selection.start),
        };
        if m.is_empty() {
            return;
        }
        self.find.current = idx;
        self.reveal_current_match();
        cx.notify();
    }

    fn refresh_matches(&mut self) {
        self.find.matches = search::find_all(self.buffer.text(), &self.find.query);
        if self.find.current.is_some_and(|i| i >= self.find.matches.len()) {
            self.find.current = None;
        }
    }

    fn reveal_current_match(&mut self) {
        if let Some(m) = self.find.current.and_then(|i| self.find.matches.get(i)).cloned() {
            self.selection = m;
            self.reversed = false;
            self.need_scroll = true;
        }
    }

    // ---- selection ----------------------------------------------------------

    fn cursor(&self) -> usize {
        if self.reversed { self.selection.start } else { self.selection.end }
    }

    fn set_cursor(&mut self, offset: usize, select: bool, cx: &mut Context<Self>) {
        let offset = offset.min(self.buffer.len());
        if select {
            let anchor = if self.reversed { self.selection.end } else { self.selection.start };
            self.reversed = offset < anchor;
            self.selection = offset.min(anchor)..offset.max(anchor);
        } else {
            self.selection = offset..offset;
            self.reversed = false;
        }
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
            self.buffer.len()
        } else {
            layout.offset_for_point(point(goal, y))
        };
        self.set_cursor(target, select, cx);
    }

    fn current_layout(&self) -> Option<Rc<Layout>> {
        self.layout_cache.borrow().as_ref().map(|(_, l)| l.clone())
    }

    /// Cached layout for the given width; rebuilt when text, style or caret line changes.
    pub(crate) fn layout_for(&mut self, width: Pixels, window: &mut Window, cx: &mut App) -> Rc<Layout> {
        let settings = cx.global::<Settings>().clone();
        let theme = *cx.global::<Theme>();
        let active_line = self.analysis.line_at(self.cursor());
        let key = LayoutKey {
            revision: self.buffer.revision(),
            width: f32::from(width).to_bits(),
            font_size: settings.font_size.to_bits(),
            font: settings.font,
            dark: theme.dark,
            active_line,
        };
        if let Some((k, l)) = self.layout_cache.borrow().as_ref()
            && *k == key
        {
            return l.clone();
        }
        let layout = Rc::new(layout::build(window, self.buffer.text(), &self.analysis, &settings, &theme, active_line, width));
        *self.layout_cache.borrow_mut() = Some((key, layout.clone()));
        layout
    }

    // ---- editing ------------------------------------------------------------

    fn edit(&mut self, range: Range<usize>, text: &str, cx: &mut Context<Self>) {
        let before = self.selection.clone();
        let cursor = self.buffer.replace(range, text, before);
        self.selection = cursor..cursor;
        self.reversed = false;
        self.marked = None;
        self.after_edit(cx);
    }

    fn after_edit(&mut self, cx: &mut Context<Self>) {
        self.analysis = markdown::analyze(self.buffer.text());
        if !self.find.query.is_empty() {
            self.refresh_matches();
        }
        self.goal_x = None;
        self.need_scroll = true;
        cx.notify();
    }

    fn delete_range_or(&mut self, fallback: Range<usize>, cx: &mut Context<Self>) {
        let r = if self.selection.is_empty() { fallback } else { self.selection.clone() };
        if !r.is_empty() {
            self.edit(r, "", cx);
        }
    }

    fn insert(&mut self, text: &str, cx: &mut Context<Self>) {
        self.edit(self.selection.clone(), text, cx);
    }

    // ---- action handlers ----------------------------------------------------

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let t = if self.selection.is_empty() { self.buffer.prev_grapheme(self.cursor()) } else { self.selection.start };
        self.horizontal(t, false, cx);
    }
    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let t = if self.selection.is_empty() { self.buffer.next_grapheme(self.cursor()) } else { self.selection.end };
        self.horizontal(t, false, cx);
    }
    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(-1, false, cx);
    }
    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(1, false, cx);
    }
    fn word_left(&mut self, _: &WordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.buffer.prev_word(self.cursor()), false, cx);
    }
    fn word_right(&mut self, _: &WordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.buffer.next_word(self.cursor()), false, cx);
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
        self.horizontal(self.buffer.len(), false, cx);
    }
    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.buffer.prev_grapheme(self.cursor()), true, cx);
    }
    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.buffer.next_grapheme(self.cursor()), true, cx);
    }
    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(-1, true, cx);
    }
    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        self.vertical(1, true, cx);
    }
    fn select_word_left(&mut self, _: &SelectWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.buffer.prev_word(self.cursor()), true, cx);
    }
    fn select_word_right(&mut self, _: &SelectWordRight, _: &mut Window, cx: &mut Context<Self>) {
        self.horizontal(self.buffer.next_word(self.cursor()), true, cx);
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
        self.horizontal(self.buffer.len(), true, cx);
    }
    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selection = 0..self.buffer.len();
        self.reversed = false;
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
        if start { self.buffer.line_start(cursor) } else { self.buffer.line_end(cursor) }
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        let c = self.cursor();
        self.delete_range_or(self.buffer.prev_grapheme(c)..c, cx);
    }
    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        let c = self.cursor();
        self.delete_range_or(c..self.buffer.next_grapheme(c), cx);
    }
    fn delete_word_left(&mut self, _: &DeleteWordLeft, _: &mut Window, cx: &mut Context<Self>) {
        let c = self.cursor();
        self.delete_range_or(self.buffer.prev_word(c)..c, cx);
    }
    fn delete_to_line_start(&mut self, _: &DeleteToLineStart, _: &mut Window, cx: &mut Context<Self>) {
        let c = self.cursor();
        let ls = self.buffer.line_start(c);
        let from = if ls == c { self.buffer.prev_grapheme(c) } else { ls };
        self.delete_range_or(from..c, cx);
    }

    fn newline(&mut self, _: &Newline, _: &mut Window, cx: &mut Context<Self>) {
        let c = self.cursor();
        if self.selection.is_empty() && c == self.buffer.line_end(c) {
            let ls = self.buffer.line_start(c);
            match markdown::continuation(&self.text()[ls..c]) {
                Some(Continuation::Clear(_)) => {
                    self.edit(ls..c, "", cx);
                    return;
                }
                Some(Continuation::Continue(prefix)) => {
                    self.insert(&format!("\n{prefix}"), cx);
                    return;
                }
                None => {}
            }
        }
        // Keep the current line's indentation.
        let ls = self.buffer.line_start(self.selection.start);
        let indent: String = self.text()[ls..].chars().take_while(|c| *c == ' ').collect();
        let indent = &indent[..indent.len().min(self.selection.start - ls)];
        self.insert(&format!("\n{indent}"), cx);
    }

    fn tab(&mut self, _: &Tab, _: &mut Window, cx: &mut Context<Self>) {
        self.insert("    ", cx);
    }

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(t) = self.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(t.to_string()));
        }
    }
    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(t) = self.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(t.to_string()));
            self.edit(self.selection.clone(), "", cx);
        }
    }
    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(t) = cx.read_from_clipboard().and_then(|i| i.text()) {
            self.insert(&t, cx);
        }
    }
    fn undo(&mut self, _: &Undo, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(sel) = self.buffer.undo() {
            self.restore(sel, cx);
        }
    }
    fn redo(&mut self, _: &Redo, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(sel) = self.buffer.redo() {
            self.restore(sel, cx);
        }
    }
    fn restore(&mut self, sel: Range<usize>, cx: &mut Context<Self>) {
        let len = self.buffer.len();
        self.selection = sel.start.min(len)..sel.end.min(len);
        self.reversed = false;
        self.marked = None;
        self.after_edit(cx);
    }

    // ---- mouse --------------------------------------------------------------

    fn offset_at(&self, p: Point<Pixels>) -> usize {
        match self.current_layout() {
            Some(l) => l.offset_for_point(p - self.origin),
            None => 0,
        }
    }

    fn mouse_down(&mut self, e: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle);
        self.selecting = true;
        let off = self.offset_at(e.position);
        self.goal_x = None;
        match e.click_count {
            2 => {
                self.selection = self.buffer.word_range(off);
                self.reversed = false;
                cx.notify();
            }
            n if n >= 3 => {
                let end = (self.buffer.line_end(off) + 1).min(self.buffer.len());
                self.selection = self.buffer.line_start(off)..end;
                self.reversed = false;
                cx.notify();
            }
            _ => self.set_cursor(off, e.modifiers.shift, cx),
        }
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
        Some(UTF16Selection { range: self.range_to_utf16(&self.selection), reversed: self.reversed })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked.as_ref().map(|r| self.range_to_utf16(r))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked = None;
    }

    fn replace_text_in_range(&mut self, range_utf16: Option<Range<usize>>, text: &str, _: &mut Window, cx: &mut Context<Self>) {
        let range = range_utf16
            .as_ref()
            .map(|r| self.range_from_utf16(r))
            .or(self.marked.clone())
            .unwrap_or(self.selection.clone());
        self.edit(range, text, cx);
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
            .or(self.marked.clone())
            .unwrap_or(self.selection.clone());
        self.edit(range.clone(), text, cx);
        if !text.is_empty() {
            self.marked = Some(range.start..range.start + text.len());
        }
        if let Some(sel) = new_selected_utf16 {
            // Offsets are relative to the inserted text.
            let local = {
                let base = &self.text()[range.start..];
                let conv = |u: usize| {
                    let (mut b, mut n) = (0, 0);
                    for ch in base.chars() {
                        if n >= u {
                            break;
                        }
                        n += ch.len_utf16();
                        b += ch.len_utf8();
                    }
                    b.min(text.len())
                };
                conv(sel.start)..conv(sel.end)
            };
            self.selection = range.start + local.start..range.start + local.end;
        }
        cx.notify();
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

impl Render for Editor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                        .pb(px(200.))
                        .child(EditorElement { editor: cx.entity() }),
                ),
            )
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
        self.selection.clone()
    }
    pub(crate) fn marked(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }
}
