//! The headless editing model: buffer, selection, decorations and every key
//! behaviour. The GPUI `Editor` is a thin shell around it, and the test harness
//! drives it directly with simulated keystrokes.
//!
//! Decorations are a pure function of (text, caret, reveal state): after every
//! mutation `sync` recomputes whatever the change could have invalidated, and
//! `scratch_views` recomputes everything from nothing so tests can compare.

use super::buffer::{Buffer, next_grapheme_in, next_word_in, prev_grapheme_in, prev_word_in};
use crate::conceal::{self, LineView, Reveal};
use crate::markdown::{self, Analysis, CodeRole, Continuation, Lead, LineKind};
use crate::shortcuts;
use crate::slash;
use std::ops::Range;
use std::rc::Rc;

/// An open `/` menu: `start` is the buffer offset of the slash.
pub struct Slash {
    pub start: usize,
    pub selected: usize,
}

/// Whether an edit keeps the current reveal state (a Backspace deleting inside
/// the revealed marker) or clears it (anything typed).
#[derive(Clone, Copy, PartialEq, Eq)]
enum EditKind {
    Typed,
    Backspace,
}

pub struct Model {
    pub buffer: Buffer,
    pub selection: Range<usize>,
    pub reversed: bool,
    pub marked: Option<Range<usize>>,
    pub analysis: Analysis,
    pub views: Rc<Vec<LineView>>,
    /// Bumped whenever `views` is rebuilt (layout cache key).
    pub views_gen: u64,
    pub reveal: Option<Reveal>,
    pub slash: Option<Slash>,
    /// Start of the caret's line when the decorations were last computed.
    caret_line_start: usize,
    /// The document contains fence characters, so the caret's line can change decorations.
    fence_sensitive: bool,
}

impl Model {
    pub fn new(text: impl Into<String>) -> Self {
        let buffer = Buffer::new(text);
        let analysis = markdown::analyze(buffer.text(), 0);
        let views = Rc::new(conceal::build_views(buffer.text(), &analysis, &None, 0));
        let mut m = Self {
            buffer,
            selection: 0..0,
            reversed: false,
            marked: None,
            analysis,
            views,
            views_gen: 0,
            reveal: None,
            slash: None,
            caret_line_start: 0,
            fence_sensitive: false,
        };
        m.fence_sensitive = has_fence_chars(m.text());
        m
    }

    /// Replaces the document (used by New and Open).
    pub fn load(&mut self, text: String) {
        self.buffer.reset(text);
        self.selection = 0..0;
        self.reversed = false;
        self.marked = None;
        self.reveal = None;
        self.slash = None;
        self.after_edit();
    }

    pub fn text(&self) -> &str {
        self.buffer.text()
    }

    pub fn cursor(&self) -> usize {
        if self.reversed { self.selection.start } else { self.selection.end }
    }

    // ---- decorations --------------------------------------------------------

    /// Brings `analysis`, `reveal` and `views` up to date after `text_changed`
    /// and/or a caret move, rebuilding only what could have changed.
    fn sync(&mut self, text_changed: bool) {
        let caret = self.cursor().min(self.buffer.len());
        let line_start = self.buffer.line_start(caret);
        let moved = line_start != self.caret_line_start;
        let mut rebuild = text_changed;
        if text_changed || (moved && self.fence_sensitive) {
            self.analysis = markdown::analyze(self.text(), caret);
            self.fence_sensitive = has_fence_chars(self.text());
            rebuild = true;
        }
        if moved && self.fence_sensitive {
            rebuild = true;
        }
        if let Some(r) = &self.reveal
            && !r.is_valid(&self.analysis, caret)
        {
            self.reveal = None;
            rebuild = true;
        }
        self.caret_line_start = line_start;
        if rebuild {
            self.rebuild_views();
        }
        self.sync_slash();
    }

    fn rebuild_views(&mut self) {
        let caret = self.cursor().min(self.buffer.len());
        self.views = Rc::new(conceal::build_views(self.text(), &self.analysis, &self.reveal, caret));
        self.views_gen += 1;
    }

    /// The decorations recomputed from nothing, for comparison with `views`.
    #[cfg(test)]
    pub fn scratch_views(&self) -> Vec<LineView> {
        let caret = self.cursor().min(self.buffer.len());
        let a = markdown::analyze(self.text(), caret);
        conceal::build_views(self.text(), &a, &self.reveal, caret)
    }

    /// The visible text without markdown markers or decoration (for counts).
    pub fn plain_text(&self) -> String {
        conceal::plain_text(&self.views)
    }

    pub fn view_at(&self, offset: usize) -> (usize, &LineView) {
        let i = self.analysis.line_at(offset).min(self.views.len() - 1);
        (i, &self.views[i])
    }

    /// Moves a buffer offset out of concealed text onto the nearest valid caret position.
    pub fn normalize(&self, offset: usize) -> usize {
        let offset = offset.min(self.buffer.len());
        self.view_at(offset).1.normalize(offset)
    }

    // ---- cursor movement ----------------------------------------------------

    pub fn prev_pos(&self, c: usize) -> usize {
        let (i, v) = self.view_at(c);
        let d = v.buf_to_disp(c);
        if d > v.min_disp {
            v.disp_to_buf(prev_grapheme_in(&v.display, d).max(v.min_disp))
        } else if i > 0 {
            let pv = &self.views[i - 1];
            pv.disp_to_buf(pv.display.len())
        } else {
            v.normalize(c)
        }
    }

    pub fn next_pos(&self, c: usize) -> usize {
        let (i, v) = self.view_at(c);
        let d = v.buf_to_disp(c);
        if d < v.display.len() {
            v.disp_to_buf(next_grapheme_in(&v.display, d))
        } else if i + 1 < self.views.len() {
            let nv = &self.views[i + 1];
            nv.disp_to_buf(nv.min_disp)
        } else {
            c
        }
    }

    pub fn prev_word_pos(&self, c: usize) -> usize {
        let (_, v) = self.view_at(c);
        let d = v.buf_to_disp(c);
        if d > v.min_disp {
            v.disp_to_buf(prev_word_in(&v.display, d).max(v.min_disp))
        } else {
            self.prev_pos(c)
        }
    }

    pub fn next_word_pos(&self, c: usize) -> usize {
        let (_, v) = self.view_at(c);
        let d = v.buf_to_disp(c);
        if d < v.display.len() {
            v.disp_to_buf(next_word_in(&v.display, d))
        } else {
            self.next_pos(c)
        }
    }

    /// Start/end of the caret's logical line (after any marker).
    pub fn logical_line_edge(&self, start: bool) -> usize {
        let (_, v) = self.view_at(self.cursor());
        if start { v.content_start() } else { v.disp_to_buf(v.display.len()) }
    }

    pub fn set_cursor(&mut self, offset: usize, select: bool) {
        let offset = self.normalize(offset);
        if select {
            let anchor = if self.reversed { self.selection.end } else { self.selection.start };
            self.reversed = offset < anchor;
            self.selection = offset.min(anchor)..offset.max(anchor);
        } else {
            self.selection = offset..offset;
            self.reversed = false;
        }
        self.sync(false);
    }

    pub fn move_left(&mut self, select: bool) {
        let t = if self.selection.is_empty() || select { self.prev_pos(self.cursor()) } else { self.selection.start };
        self.set_cursor(t, select);
    }

    pub fn move_right(&mut self, select: bool) {
        let t = if self.selection.is_empty() || select { self.next_pos(self.cursor()) } else { self.selection.end };
        self.set_cursor(t, select);
    }

    pub fn select_all(&mut self) {
        self.selection = 0..self.buffer.len();
        self.reversed = false;
        self.sync(false);
    }

    /// Moves one logical line up/down keeping the display column (unwrapped
    /// approximation of the layout-based vertical movement).
    #[cfg(test)]
    pub fn move_vertical(&mut self, dir: i32, select: bool) {
        let c = self.cursor();
        let (i, v) = self.view_at(c);
        let col = v.buf_to_disp(c).saturating_sub(v.min_disp);
        let j = if dir < 0 { i.checked_sub(1) } else { (i + 1 < self.views.len()).then_some(i + 1) };
        let target = match j {
            None => if dir < 0 { 0 } else { self.buffer.len() },
            Some(j) => {
                let t = &self.views[j];
                let mut d = (t.min_disp + col).min(t.display.len());
                while !t.display.is_char_boundary(d) {
                    d -= 1;
                }
                t.disp_to_buf(d)
            }
        };
        self.set_cursor(target, select);
    }

    // ---- editing ------------------------------------------------------------

    fn edit(&mut self, range: Range<usize>, text: &str, kind: EditKind) {
        self.edit_caret(range, text, None, kind);
    }

    fn edit_caret(&mut self, range: Range<usize>, text: &str, caret: Option<usize>, kind: EditKind) {
        let before = self.selection.clone();
        let (at, removed) = (range.start, range.len());
        let c = self.buffer.replace(range, text, before);
        match (&mut self.reveal, kind) {
            (Some(r), EditKind::Backspace) => r.adjust(at, removed, text.len()),
            _ => self.reveal = None,
        }
        let c = caret.unwrap_or(c);
        self.selection = c..c;
        self.reversed = false;
        self.marked = None;
        self.after_edit();
    }

    fn after_edit(&mut self) {
        self.sync(true);
        if self.selection.is_empty() {
            let c = self.normalize(self.selection.end);
            self.selection = c..c;
        }
        self.sync(false);
    }

    fn delete_range_or(&mut self, fallback: Range<usize>, kind: EditKind) {
        let r = if self.selection.is_empty() { fallback } else { self.selection.clone() };
        if !r.is_empty() {
            self.edit(r, "", kind);
        }
    }

    /// Replaces `range` with `text` as typed input, without shortcuts (IME composition).
    pub fn insert_range(&mut self, range: Range<usize>, text: &str) {
        self.edit(range, text, EditKind::Typed);
    }

    pub fn set_selection(&mut self, sel: Range<usize>) {
        self.selection = sel;
        self.reversed = false;
        self.sync(false);
    }

    /// Inserts text without any typing shortcuts (paste, programmatic).
    pub fn insert(&mut self, text: &str) {
        self.edit(self.selection.clone(), text, EditKind::Typed);
    }

    /// Text-input path (what the platform IME delivers): replaces `range` (or
    /// the marked text / selection) and applies typing shortcuts for one character.
    pub fn replace_text(&mut self, range: Option<Range<usize>>, text: &str) {
        let range = range.or(self.marked.clone()).unwrap_or(self.selection.clone());
        self.edit(range, text, EditKind::Typed);
        if text.chars().count() == 1 {
            let c = self.cursor();
            let in_code = self.analysis.lines[self.analysis.line_at(c)].kind == LineKind::Code;
            if let Some(e) = shortcuts::after_insert(self.text(), c, text, in_code) {
                self.edit_caret(e.range, &e.text, Some(e.caret), EditKind::Typed);
            } else if text == "/" && !in_code && slash::should_open(self.text(), c) {
                self.slash = Some(Slash { start: c - 1, selected: 0 });
                self.sync_slash();
            }
        }
    }

    /// Types `text` one character at a time through the text-input path; a
    /// newline is the Enter key.
    #[cfg(test)]
    pub fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            if ch == '\n' {
                self.enter();
            } else {
                self.replace_text(None, &ch.to_string());
            }
        }
    }

    // ---- Backspace / Delete -------------------------------------------------

    /// Whether joining line `prev` with the line after it (deleting the newline
    /// between them) would corrupt a code fence.
    fn join_blocked(&self, prev: usize) -> bool {
        let (p, n) = (&self.analysis.lines[prev], &self.analysis.lines[prev + 1]);
        let (p_empty, n_empty) = (p.start == p.end, n.start == n.end);
        if p_empty || n_empty {
            return false;
        }
        let blocks = (p.code.as_ref().map(|c| c.block), n.code.as_ref().map(|c| c.block));
        blocks.0 != blocks.1 || p.is_fence() || n.is_fence()
    }

    /// Removes the empty fenced block whose single empty body line is `li`.
    fn remove_empty_block(&mut self, li: usize) -> bool {
        let a = &self.analysis;
        let line = &a.lines[li];
        let is_body = line.code.as_ref().is_some_and(|c| c.role == CodeRole::Body);
        let Some((open, Some(close))) = a.block_lines(li) else { return false };
        if !is_body || line.start != line.end || close != open + 2 || li != open + 1 {
            return false;
        }
        let range = a.lines[open].start..a.lines[close].end;
        let at = range.start;
        self.edit_caret(range, "", Some(at), EditKind::Typed);
        true
    }

    pub fn backspace(&mut self) {
        let c = self.cursor();
        if !self.selection.is_empty() {
            self.delete_range_or(c..c, EditKind::Backspace);
            return;
        }
        // First Backspace into a concealed marker reveals it instead of deleting.
        if let Some(r) = conceal::on_backspace(&self.analysis, c, &self.reveal) {
            self.reveal = Some(r);
            self.rebuild_views();
            // Switching reveals can conceal text the caret was inside of.
            let n = self.normalize(c);
            self.selection = n..n;
            self.sync(false);
            return;
        }
        let (li, v) = self.view_at(c);
        if v.buf_to_disp(c) <= v.min_disp && li > 0 {
            if self.remove_empty_block(li) {
                return;
            }
            if self.join_blocked(li - 1) {
                let end = self.analysis.lines[li - 1].end;
                self.set_cursor(end, false);
                return;
            }
        }
        let p = self.prev_pos(c);
        self.delete_range_or(p..c, EditKind::Backspace);
    }

    pub fn delete(&mut self) {
        let c = self.cursor();
        if self.selection.is_empty() {
            let (li, v) = self.view_at(c);
            if v.buf_to_disp(c) >= v.display.len() && li + 1 < self.views.len() && self.join_blocked(li) {
                return;
            }
        }
        self.delete_range_or(c..self.next_pos(c), EditKind::Typed);
    }

    pub fn delete_word_left(&mut self) {
        let c = self.cursor();
        let (li, v) = self.view_at(c);
        if self.selection.is_empty() && v.buf_to_disp(c) <= v.min_disp && li > 0 {
            self.backspace();
            return;
        }
        self.delete_range_or(self.prev_word_pos(c)..c, EditKind::Backspace);
    }

    pub fn delete_to_line_start(&mut self) {
        let c = self.cursor();
        let cs = self.view_at(c).1.content_start();
        let from = if cs < c { cs } else { self.prev_pos(c) };
        self.delete_range_or(from..c, EditKind::Backspace);
    }

    // ---- Enter / Tab --------------------------------------------------------

    /// The text to insert (and caret offset inside it) when Enter closes a
    /// freshly typed opening fence at the end of its line.
    fn fence_autoclose(&self, c: usize) -> Option<(String, usize)> {
        let li = self.analysis.line_at(c);
        let line = &self.analysis.lines[li];
        if c != line.end {
            return None;
        }
        let (indent, fence) = markdown::fence_opener(&self.text()[line.start..line.end])?;
        // With this line forced to plain text it must sit outside any block, and
        // every other block must be closed: otherwise it pairs with a later fence.
        let probe = markdown::analyze_plain_line(self.text(), line.start);
        if probe.lines[li].code.is_some() || !probe.unclosed.is_empty() {
            return None;
        }
        Some((format!("\n{indent}\n{indent}{fence}"), 1 + indent.len()))
    }

    pub fn enter(&mut self) {
        if let Some(i) = self.slash.as_ref().map(|s| s.selected) {
            self.slash_pick(i);
            return;
        }
        let c = self.cursor();
        let li = self.analysis.line_at(c);
        let line = self.analysis.lines[li].clone();
        if self.selection.is_empty() {
            if let Some((ins, off)) = self.fence_autoclose(c) {
                self.edit_caret(c..c, &ins, Some(c + off), EditKind::Typed);
                return;
            }
            if line.code.is_none()
                && let Some(lead) = &line.lead
                && c >= lead.content_start(&line)
                && let Some(cont) = markdown::continuation(self.text(), &line)
            {
                match cont {
                    Continuation::Clear if c == line.end => {
                        self.edit(line.start..c, "", EditKind::Typed);
                        return;
                    }
                    Continuation::Continue(prefix) => {
                        self.insert(&format!("\n{prefix}"));
                        return;
                    }
                    Continuation::Clear => {}
                }
            }
        }
        // Keep the current line's indentation.
        let ls = self.buffer.line_start(self.selection.start);
        let indent: String = self.text()[ls..].chars().take_while(|c| *c == ' ').collect();
        let indent = &indent[..indent.len().min(self.selection.start - ls)];
        self.insert(&format!("\n{indent}"));
    }

    fn indent_of(&self, li: usize) -> usize {
        let l = &self.analysis.lines[li];
        self.text()[l.start..l.end].bytes().take_while(|b| *b == b' ').count()
    }

    pub fn tab(&mut self) {
        let c = self.cursor();
        let li = self.analysis.line_at(c);
        let line = &self.analysis.lines[li];
        let is_item = self.selection.is_empty() && line.code.is_none() && line.lead.as_ref().is_some_and(Lead::is_list);
        if !is_item {
            self.insert("    ");
            return;
        }
        // Nest under the previous item at the same level: the marker moves to
        // that item's content column.
        let have = self.indent_of(li);
        let sibling = (0..li)
            .rev()
            .map(|j| &self.analysis.lines[j])
            .take_while(|l| l.start != l.end)
            .filter(|l| l.lead.as_ref().is_some_and(Lead::is_list))
            .map(|l| (self.indent_of(self.analysis.line_at(l.start)), l))
            .find(|(indent, _)| *indent <= have);
        let Some((indent, sib)) = sibling else { return };
        let Some(sm) = sib.marker().filter(|_| indent == have) else { return };
        let want = sm.end - sib.start;
        let (ls, marker) = (line.start, line.marker().expect("list lines have a marker"));
        let old = &self.text()[ls..marker.end];
        let mut new = " ".repeat(want - have) + old;
        if let Some(Lead::Ordered { .. }) = line.lead {
            // A nested numbered list starts over.
            let ds = new.find(|ch: char| ch.is_ascii_digit()).unwrap_or(0);
            let de = ds + new[ds..].bytes().take_while(u8::is_ascii_digit).count();
            new.replace_range(ds..de, "1");
        }
        let caret = if c >= marker.end { c + new.len() - old.len() } else { ls + new.len() };
        self.edit_caret(ls..marker.end, &new, Some(caret), EditKind::Typed);
    }

    pub fn backtab(&mut self) {
        if !self.selection.is_empty() {
            return;
        }
        let c = self.cursor();
        let li = self.analysis.line_at(c);
        let line = &self.analysis.lines[li];
        let have = self.indent_of(li);
        let n = if line.code.is_none() && line.lead.as_ref().is_some_and(Lead::is_list) {
            // Outdent to the enclosing item's indentation.
            let parent = (0..li)
                .rev()
                .take_while(|j| self.analysis.lines[*j].start != self.analysis.lines[*j].end)
                .find(|j| self.analysis.lines[*j].lead.as_ref().is_some_and(Lead::is_list) && self.indent_of(*j) < have)
                .map(|j| self.indent_of(j))
                .unwrap_or(0);
            have - parent
        } else {
            have.min(4)
        };
        if n > 0 {
            let ls = line.start;
            self.edit_caret(ls..ls + n, "", Some(c.saturating_sub(n).max(ls)), EditKind::Typed);
        }
    }

    pub fn toggle_task(&mut self, li: usize) {
        let lead = self.analysis.lines.get(li).and_then(|l| l.lead.clone());
        if let Some(Lead::Task { bracket, checked, .. }) = lead {
            let c = self.cursor();
            self.edit_caret(bracket.start + 1..bracket.start + 2, if checked { " " } else { "x" }, Some(c), EditKind::Typed);
        }
    }

    pub fn toggle_check(&mut self) {
        let li = self.analysis.line_at(self.cursor());
        self.toggle_task(li);
    }

    // ---- clipboard / history ------------------------------------------------

    pub fn selected_text(&self) -> Option<&str> {
        (!self.selection.is_empty()).then(|| &self.text()[self.selection.clone()])
    }

    pub fn cut(&mut self) -> Option<String> {
        let t = self.selected_text()?.to_string();
        self.edit(self.selection.clone(), "", EditKind::Typed);
        Some(t)
    }

    pub fn undo(&mut self) {
        if let Some(sel) = self.buffer.undo() {
            self.restore(sel);
        }
    }

    pub fn redo(&mut self) {
        if let Some(sel) = self.buffer.redo() {
            self.restore(sel);
        }
    }

    fn restore(&mut self, sel: Range<usize>) {
        let len = self.buffer.len();
        self.selection = sel.start.min(len)..sel.end.min(len);
        self.reversed = false;
        self.marked = None;
        self.reveal = None;
        self.slash = None;
        self.after_edit();
    }

    // ---- slash menu ---------------------------------------------------------

    pub fn slash_query(&self) -> Option<String> {
        let s = self.slash.as_ref()?;
        slash::query(self.text(), s.start, self.selection.end).map(str::to_string)
    }

    fn sync_slash(&mut self) {
        if self.slash.is_none() {
            return;
        }
        let n = self.slash_query().map(|q| slash::filter(&q).len()).unwrap_or(0);
        match &mut self.slash {
            Some(s) if n > 0 && self.selection.is_empty() => s.selected = s.selected.min(n - 1),
            _ => self.slash = None,
        }
    }

    pub fn slash_move(&mut self, dir: i32) {
        let n = self.slash_query().map(|q| slash::filter(&q).len()).unwrap_or(1).max(1);
        if let Some(s) = &mut self.slash {
            s.selected = (s.selected as i32 + dir).rem_euclid(n as i32) as usize;
        }
    }

    pub fn slash_pick(&mut self, index: usize) {
        let Some(start) = self.slash.as_ref().map(|s| s.start) else { return };
        let Some(q) = self.slash_query() else { return };
        let Some(item) = slash::filter(&q).get(index).copied() else { return };
        let end = self.selection.end;
        let (text, caret) = slash::replacement(item.block, self.text(), start);
        self.slash = None;
        self.edit_caret(start..end, &text, Some(start + caret), EditKind::Typed);
    }
}

fn has_fence_chars(text: &str) -> bool {
    text.contains("```") || text.contains("~~~")
}
