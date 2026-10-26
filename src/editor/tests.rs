//! Headless editor tests: a harness drives the model with simulated keystrokes
//! and asserts on buffer text, caret and the computed decorations.
//!
//! Every harness operation re-checks the global invariants, so each test is also
//! a consistency test: cached decorations always equal decorations computed from
//! scratch, the caret never sits in concealed text, and a reveal is always valid.

use super::model::Model;
use crate::conceal::{LineView, SegKind};
use crate::markdown::LineKind;

struct H {
    m: Model,
}

impl H {
    fn new() -> Self {
        Self { m: Model::new("") }
    }

    /// A document with the caret at the end.
    fn with(text: &str) -> Self {
        let mut h = Self { m: Model::new(text) };
        h.doc_end();
        h
    }

    fn check(&self) {
        let m = &self.m;
        let t = m.text();
        let c = m.cursor();
        assert!(m.selection.start <= m.selection.end && m.selection.end <= t.len(), "selection {:?} in {t:?}", m.selection);
        assert!(t.is_char_boundary(m.selection.start) && t.is_char_boundary(m.selection.end));
        assert_eq!(*m.views, m.scratch_views(), "stale decorations for {t:?} caret {c}");
        assert_eq!(m.views.len(), t.matches('\n').count() + 1);
        if m.selection.is_empty() {
            assert_eq!(m.normalize(c), c, "caret {c} inside concealed text in {t:?}");
        }
        if let Some(r) = &m.reveal {
            assert!(r.is_valid(&m.analysis, c), "stale reveal {r:?} in {t:?} caret {c}");
        }
    }

    // ---- keystrokes ---------------------------------------------------------

    /// Types through the text-input path; `\n` is the Enter key.
    fn type_(&mut self, s: &str) -> &mut Self {
        for ch in s.chars() {
            self.m.type_text(&ch.to_string());
            self.check();
        }
        self
    }

    /// Types every character (newlines too) as plain text input, no Enter semantics.
    fn raw(&mut self, s: &str) -> &mut Self {
        for ch in s.chars() {
            self.m.replace_text(None, &ch.to_string());
            self.check();
        }
        self
    }

    fn paste(&mut self, s: &str) -> &mut Self {
        self.m.insert(s);
        self.check();
        self
    }

    fn bs(&mut self, n: usize) -> &mut Self {
        for _ in 0..n {
            self.m.backspace();
            self.check();
        }
        self
    }

    fn del(&mut self) -> &mut Self {
        self.m.delete();
        self.check();
        self
    }

    fn enter(&mut self) -> &mut Self {
        self.m.enter();
        self.check();
        self
    }

    fn tab(&mut self) -> &mut Self {
        self.m.tab();
        self.check();
        self
    }

    fn backtab(&mut self) -> &mut Self {
        self.m.backtab();
        self.check();
        self
    }

    fn toggle(&mut self) -> &mut Self {
        self.m.toggle_check();
        self.check();
        self
    }

    fn left(&mut self, n: usize) -> &mut Self {
        for _ in 0..n {
            self.m.move_left(false);
            self.check();
        }
        self
    }

    fn right(&mut self, n: usize) -> &mut Self {
        for _ in 0..n {
            self.m.move_right(false);
            self.check();
        }
        self
    }

    fn up(&mut self) -> &mut Self {
        self.m.move_vertical(-1, false);
        self.check();
        self
    }

    fn down(&mut self) -> &mut Self {
        self.m.move_vertical(1, false);
        self.check();
        self
    }

    fn home(&mut self) -> &mut Self {
        self.m.set_cursor(self.m.logical_line_edge(true), false);
        self.check();
        self
    }

    fn end(&mut self) -> &mut Self {
        self.m.set_cursor(self.m.logical_line_edge(false), false);
        self.check();
        self
    }

    fn doc_end(&mut self) -> &mut Self {
        self.m.set_cursor(self.m.buffer.len(), false);
        self.check();
        self
    }

    fn doc_start(&mut self) -> &mut Self {
        self.m.set_cursor(0, false);
        self.check();
        self
    }

    fn undo(&mut self) -> &mut Self {
        self.m.undo();
        self.check();
        self
    }

    fn redo(&mut self) -> &mut Self {
        self.m.redo();
        self.check();
        self
    }

    // ---- observations -------------------------------------------------------

    /// The buffer text with `|` at the caret.
    fn s(&self) -> String {
        let c = self.m.cursor();
        format!("{}|{}", &self.m.text()[..c], &self.m.text()[c..])
    }

    fn text(&self) -> &str {
        self.m.text()
    }

    fn views(&self) -> &[LineView] {
        &self.m.views
    }

    /// One entry per line: `kind:display` (glyph prefix spaces shown as `_`/`~`).
    fn deco(&self) -> Vec<String> {
        deco(self.views())
    }

    /// The concealed buffer text of every line.
    fn hidden(&self) -> Vec<String> {
        self.views()
            .iter()
            .map(|v| {
                v.segs
                    .iter()
                    .filter(|s| s.kind == SegKind::Concealed)
                    .map(|s| &self.m.text()[s.buf.clone()])
                    .collect::<String>()
            })
            .collect()
    }
}

fn deco(views: &[LineView]) -> Vec<String> {
    views
        .iter()
        .map(|v| {
            let k = match v.kind {
                LineKind::Normal => "p".to_string(),
                LineKind::Heading(n) => format!("h{n}"),
                LineKind::Quote => "q".to_string(),
                LineKind::Code if v.fence_hidden => "fence".to_string(),
                LineKind::Code => "code".to_string(),
                LineKind::Rule => "hr".to_string(),
            };
            format!("{k}:{}", v.display.replace('\u{2002}', "_").replace('\u{2003}', "~"))
        })
        .collect()
}

fn strs(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// Snippets covering every construct. Each is a complete document.
const CORPUS: &[&str] = &[
    "# H1",
    "## H2 words",
    "### H3",
    "#### H4 styled as h3",
    "# ",
    "#nospace",
    "- item",
    "* star",
    "+ plus",
    "1. one\n2. two",
    "3) paren",
    "- [ ] todo",
    "- [x] done",
    "- [X] DONE",
    "> quote",
    "> a\n> b",
    "---",
    "para\n\n---\n\nafter",
    "text\n---",
    "**bold**",
    "*it*",
    "_it_",
    "`code`",
    "~~gone~~",
    "[link](http://x.y)",
    "a **b** and *c* and `d` and ~~e~~",
    "```\ncode\n```",
    "```rust\nfn main() {}\n```",
    "~~~\ntilde\n~~~",
    "```\n**not bold** # not heading\n```",
    "- a\n  - b\n    - c",
    "1. a\n   - x\n2. b",
    "# Title\n\npara **b**\n\n- l1\n- l2\n\n> q\n\n```\nx\n```",
    "text\n- ",
    "- a\n- ",
    "- [ ] a\n- [x] b",
    "***both***",
    "![img](p.png)",
    "```\nunclosed",
    "a `` `tick` `` b",
    "# **bold heading**",
    "> - quoted item",
];

// ---- typing vs pasting -------------------------------------------------------

#[test]
fn typing_and_pasting_give_identical_decorations() {
    for s in CORPUS {
        let mut typed = H::new();
        typed.raw(s).doc_end();
        let mut pasted = H::new();
        pasted.paste(s).doc_end();
        assert_eq!(typed.text(), *s);
        assert_eq!(typed.views(), pasted.views(), "typed vs pasted: {s:?}");
        // and a document loaded from a file
        let mut m = Model::new("");
        m.load(s.to_string());
        m.set_cursor(s.len(), false);
        assert_eq!(*m.views, *pasted.views(), "loaded vs pasted: {s:?}");
    }
}

#[test]
fn typing_with_enter_matches_paste_for_single_line_snippets() {
    for s in CORPUS.iter().filter(|s| !s.contains('\n')) {
        let mut typed = H::new();
        typed.type_(s).doc_end();
        let mut pasted = H::new();
        pasted.paste(s).doc_end();
        assert_eq!(typed.views(), pasted.views(), "{s:?}");
    }
}

#[test]
fn every_construct_actually_renders() {
    let cases: &[(&str, &[&str])] = &[
        ("# H1", &["h1:H1"]),
        ("## H2", &["h2:H2"]),
        ("### H3", &["h3:H3"]),
        ("#### H4", &["h4:H4"]),
        ("- a", &["p:•_a"]),
        ("* a", &["p:•_a"]),
        ("1. a\n2. b", &["p:1._a", "p:2._b"]),
        ("- [ ] t", &["p:~t"]),
        ("> q", &["q:q"]),
        ("a **b** c", &["p:a b c"]),
        ("a *b* c", &["p:a b c"]),
        ("a `b` c", &["p:a b c"]),
        ("a ~~b~~ c", &["p:a b c"]),
        ("a [b](u) c", &["p:a b c"]),
    ];
    for (src, want) in cases {
        assert_eq!(H::with(src).deco(), strs(want), "{src:?}");
    }
    assert_eq!(H::with("x\n\n---").deco(), strs(&["p:x", "p:", "hr:"]));
    // the fences show raw only while the caret is on them
    let mut h = H::with("```\nx\n```");
    h.doc_start().down();
    assert_eq!(h.deco(), strs(&["fence:", "code:x", "fence:"]));
}

// ---- backspace then retype ---------------------------------------------------

#[test]
fn backspace_n_times_then_retype_matches_first_typing() {
    for s in CORPUS {
        let mut first = H::new();
        first.raw(s).doc_end();
        for n in 1..=s.chars().count() + 3 {
            let mut h = H::new();
            h.raw(s).doc_end().bs(n);
            let rest = h.text().to_string();
            assert!(s.starts_with(&rest), "{s:?} after {n} backspaces left {rest:?}");
            if rest == *s {
                // Only reveals so far: nothing was deleted, nothing to retype.
                continue;
            }
            h.doc_end().raw(&s[rest.len()..]).doc_end();
            assert_eq!(h.text(), *s, "{s:?} n={n}");
            assert_eq!(h.views(), first.views(), "retype after {n} backspaces: {s:?}");
        }
    }
}

#[test]
fn heading_marker_deleted_and_retyped_detects_again() {
    let mut h = H::new();
    h.type_("# Title").home();
    assert_eq!(h.deco(), strs(&["h1:Title"]));
    h.bs(1); // reveal
    assert_eq!(h.deco(), strs(&["p:# Title"]));
    h.bs(1); // delete the space
    assert_eq!(h.s(), "#|Title");
    assert_eq!(h.deco(), strs(&["p:#Title"]));
    h.type_(" ");
    assert_eq!(h.s(), "# |Title");
    assert_eq!(h.deco(), strs(&["h1:Title"]));
    assert_eq!(h.hidden(), strs(&["# "]));
}

#[test]
fn delete_the_whole_marker_and_type_a_different_one() {
    let mut h = H::new();
    h.type_("# Title").home().bs(3);
    assert_eq!(h.s(), "|Title");
    h.type_("## ");
    assert_eq!(h.deco(), strs(&["h2:Title"]));
}

// ---- reveal state machine ----------------------------------------------------

#[test]
fn reveal_first_backspace_shows_marker_second_deletes() {
    let mut h = H::with("- item");
    h.home();
    assert_eq!(h.s(), "- |item");
    assert!(h.m.reveal.is_none());
    h.bs(1);
    assert_eq!(h.text(), "- item", "first backspace deletes nothing");
    assert_eq!(h.deco(), strs(&["p:- item"]));
    assert!(h.m.reveal.is_some());
    h.bs(1);
    assert_eq!(h.s(), "-|item");
    assert!(h.m.reveal.is_none(), "reveal cleared when its marker range no longer exists");
    assert_eq!(h.deco(), strs(&["p:-item"]));
    h.type_(" ");
    assert_eq!(h.deco(), strs(&["p:•_item"]));
}

#[test]
fn reveal_clears_when_the_caret_leaves_the_line() {
    let mut h = H::with("# A\nb");
    h.up().home().bs(1);
    assert_eq!(h.deco(), strs(&["p:# A", "p:b"]));
    h.down();
    assert!(h.m.reveal.is_none());
    assert_eq!(h.deco(), strs(&["h1:A", "p:b"]));
    // coming back does not reveal again
    h.up();
    assert_eq!(h.deco(), strs(&["h1:A", "p:b"]));
}

#[test]
fn reveal_clears_on_typing() {
    let mut h = H::with("# Title");
    h.home().bs(1);
    assert_eq!(h.deco(), strs(&["p:# Title"]));
    h.type_("x");
    assert!(h.m.reveal.is_none());
    assert_eq!(h.text(), "# xTitle");
    assert_eq!(h.deco(), strs(&["h1:xTitle"]));
}

#[test]
fn arrowing_or_clicking_into_a_line_never_reveals() {
    let mut h = H::with("# A\n- b\n\n> c\n\n**d**");
    for _ in 0..6 {
        h.up();
    }
    assert!(h.m.reveal.is_none());
    h.doc_start().right(1).left(1);
    assert!(h.m.reveal.is_none());
    assert_eq!(h.deco(), strs(&["h1:A", "p:•_b", "p:", "q:c", "p:", "p:d"]));
    h.m.set_cursor(6, false); // "click" into the bullet line
    assert!(h.m.reveal.is_none());
    assert_eq!(h.deco()[1], "p:•_b");
}

#[test]
fn caret_never_lands_in_concealed_markers() {
    let mut h = H::with("# A\n- b **c** d");
    h.doc_start();
    for _ in 0..20 {
        h.right(1);
    }
    for _ in 0..20 {
        h.left(1);
    }
    assert_eq!(h.s(), "# |A\n- b **c** d");
}

#[test]
fn inline_reveal_flow_bold() {
    let mut h = H::new();
    h.type_("**b**");
    assert_eq!(h.deco(), strs(&["p:b"]));
    h.bs(1); // reveals the span
    assert_eq!(h.deco(), strs(&["p:**b**"]));
    h.bs(1);
    assert_eq!(h.text(), "**b*");
    assert!(h.m.reveal.is_none());
    h.type_("*");
    assert_eq!(h.text(), "**b**");
    assert_eq!(h.deco(), strs(&["p:b"]));
}

#[test]
fn inline_reveal_clears_when_leaving_the_line() {
    let mut h = H::with("x\n`c`");
    h.bs(1);
    assert_eq!(h.deco(), strs(&["p:x", "p:`c`"]));
    h.up();
    assert!(h.m.reveal.is_none());
    assert_eq!(h.deco(), strs(&["p:x", "p:c"]));
}

#[test]
fn backspace_on_a_divider_reveals_then_breaks_it() {
    let mut h = H::new();
    h.type_("a\n\n---");
    assert_eq!(h.deco()[2], "hr:");
    h.bs(1);
    assert_eq!(h.deco()[2], "p:---");
    h.bs(1);
    assert_eq!(h.text(), "a\n\n--");
    assert_eq!(h.deco()[2], "p:--");
    h.type_("-");
    assert_eq!(h.deco()[2], "hr:");
}

#[test]
fn empty_bullet_and_checkbox_reveal() {
    let mut h = H::new();
    h.type_("- [ ] ");
    assert_eq!(h.deco(), strs(&["p:~"]));
    h.bs(1);
    assert_eq!(h.deco(), strs(&["p:- [ ] "]));
    h.bs(1);
    assert_eq!(h.text(), "- [ ]");
    assert_eq!(h.deco(), strs(&["p:•_[ ]"]));
}

// ---- lists -------------------------------------------------------------------

#[test]
fn enter_continues_bullets_and_empty_item_exits() {
    let mut h = H::new();
    h.type_("- a\n");
    assert_eq!(h.s(), "- a\n- |");
    assert_eq!(h.deco(), strs(&["p:•_a", "p:•_"]));
    h.type_("b\n");
    assert_eq!(h.s(), "- a\n- b\n- |");
    h.enter();
    assert_eq!(h.s(), "- a\n- b\n|");
    assert_eq!(h.deco(), strs(&["p:•_a", "p:•_b", "p:"]));
}

#[test]
fn enter_continues_numbered_lists_and_renumbers() {
    let mut h = H::new();
    h.type_("1. a\nb\nc");
    assert_eq!(h.text(), "1. a\n2. b\n3. c");
    assert_eq!(h.deco(), strs(&["p:1._a", "p:2._b", "p:3._c"]));
    // insert in the middle: later numbers shift
    h.up().end().enter().type_("new");
    assert_eq!(h.text(), "1. a\n2. b\n3. new\n3. c");
    assert_eq!(h.deco(), strs(&["p:1._a", "p:2._b", "p:3._new", "p:4._c"]));
    // delete a middle item: numbers close up
    h.m.set_selection(5..10);
    h.m.backspace();
    assert_eq!(h.deco().len(), 3);
    assert_eq!(h.deco()[2], "p:3._c");
}

#[test]
fn enter_continues_tasks_unchecked() {
    let mut h = H::new();
    h.type_("- [x] done\n");
    assert_eq!(h.s(), "- [x] done\n- [ ] |");
    assert_eq!(h.deco(), strs(&["p:~done", "p:~"]));
    assert!(h.views()[0].checkbox.as_ref().unwrap().checked);
    assert!(!h.views()[1].checkbox.as_ref().unwrap().checked);
    h.enter();
    assert_eq!(h.s(), "- [x] done\n|");
}

#[test]
fn enter_in_the_middle_of_an_item_splits_it() {
    let mut h = H::with("- ab");
    h.left(1);
    h.enter();
    assert_eq!(h.s(), "- a\n- |b");
}

#[test]
fn enter_continues_quotes_and_exits_on_empty() {
    let mut h = H::new();
    h.type_("> a\n");
    assert_eq!(h.s(), "> a\n> |");
    assert_eq!(h.deco(), strs(&["q:a", "q:"]));
    h.enter();
    assert_eq!(h.s(), "> a\n|");
    assert_eq!(h.deco(), strs(&["q:a", "p:"]));
}

#[test]
fn enter_after_heading_is_plain() {
    let mut h = H::new();
    h.type_("# A\nb");
    assert_eq!(h.deco(), strs(&["h1:A", "p:b"]));
}

#[test]
fn tab_nests_under_the_previous_item_and_shift_tab_undoes_it() {
    let mut h = H::new();
    h.type_("- a\nb");
    h.tab();
    assert_eq!(h.s(), "- a\n  - b|");
    assert_eq!(h.deco(), strs(&["p:•_a", "p:~◦_b"]));
    h.tab();
    assert_eq!(h.s(), "- a\n  - b|", "cannot nest deeper than one below the item above");
    h.type_("\nc");
    h.tab();
    assert_eq!(h.s(), "- a\n  - b\n    - c|");
    assert_eq!(h.deco()[2], "p:~~▪_c");
    h.backtab();
    assert_eq!(h.s(), "- a\n  - b\n  - c|");
    h.backtab();
    assert_eq!(h.s(), "- a\n  - b\n- c|");
    assert_eq!(h.deco(), strs(&["p:•_a", "p:~◦_b", "p:•_c"]));
}

#[test]
fn tab_on_the_first_item_is_a_no_op_and_ordered_items_nest() {
    let mut h = H::new();
    h.type_("- a");
    h.tab();
    assert_eq!(h.s(), "- a|");
    let mut h = H::new();
    h.type_("1. a\nb");
    h.tab();
    assert_eq!(h.s(), "1. a\n   1. b|", "a nested numbered list starts over");
    assert!(matches!(h.m.analysis.lines[1].lead, Some(crate::markdown::Lead::Ordered { depth: 1, number: 1, .. })));
}

#[test]
fn tab_in_plain_text_inserts_four_spaces() {
    let mut h = H::new();
    h.tab().type_("a");
    assert_eq!(h.s(), "    a|");
    assert_eq!(h.deco(), strs(&["p:    a"]), "indented text is not an indented-code panel");
    h.backtab();
    assert_eq!(h.s(), "a|");
}

#[test]
fn cmd_enter_toggles_tasks() {
    let mut h = H::new();
    h.type_("- [ ] a");
    h.toggle();
    assert_eq!(h.text(), "- [x] a");
    assert!(h.views()[0].checkbox.as_ref().unwrap().checked);
    h.toggle();
    assert_eq!(h.text(), "- [ ] a");
    h.undo();
    assert_eq!(h.text(), "- [x] a");
    // not a task: nothing happens
    let mut h = H::with("- a");
    h.toggle();
    assert_eq!(h.text(), "- a");
}

#[test]
fn bracket_shortcuts_normalize_to_a_task() {
    for typed in ["[] ", "[ ] "] {
        let mut h = H::new();
        h.type_(typed);
        assert_eq!(h.s(), "- [ ] |", "{typed:?}");
        assert_eq!(h.deco(), strs(&["p:~"]));
    }
    // not at the start of a line
    let mut h = H::new();
    h.type_("a [] ");
    assert_eq!(h.text(), "a [] ");
}

#[test]
fn empty_bullet_under_a_paragraph_shows_immediately() {
    let mut h = H::new();
    h.type_("Groceries:\n- ");
    assert_eq!(h.deco(), strs(&["p:Groceries:", "p:•_"]));
    h.type_("milk");
    assert_eq!(h.deco(), strs(&["p:Groceries:", "p:•_milk"]));
    let mut h = H::new();
    h.type_("Steps:\n1. ");
    assert_eq!(h.deco(), strs(&["p:Steps:", "p:1._"]));
}

#[test]
fn divider_under_text_is_a_divider() {
    let mut h = H::new();
    h.type_("text\n---");
    assert_eq!(h.deco(), strs(&["p:text", "hr:"]));
    h.bs(2);
    assert_eq!(h.deco(), strs(&["p:text", "p:--"]));
}

// ---- code blocks -------------------------------------------------------------

#[test]
fn fence_plus_enter_creates_a_block_with_closing_fence() {
    let mut h = H::new();
    h.type_("```rust");
    assert_eq!(h.deco(), strs(&["p:```rust"]), "while typing the fence stays plain text");
    h.enter();
    assert_eq!(h.s(), "```rust\n|\n```");
    assert_eq!(h.deco(), strs(&["fence:", "code:", "fence:"]));
    assert!(h.views()[0].fence_hidden && h.views()[2].fence_hidden);
    assert_eq!(h.hidden(), strs(&["```rust", "", "```"]));
}

#[test]
fn typing_inside_a_block_is_never_parsed() {
    let mut h = H::new();
    h.type_("```\n");
    h.type_("# not a heading\n- not a list\n**not bold** `x`\n> no\n");
    assert_eq!(h.deco(), strs(&["fence:", "code:# not a heading", "code:- not a list", "code:**not bold** `x`", "code:> no", "code:", "fence:"]));
    assert_eq!(h.m.analysis.lines[1].lead, None);
    assert!(h.views().iter().all(|v| v.code_spans.is_empty()));
}

#[test]
fn enter_in_code_keeps_indentation_and_tab_inserts_four_spaces() {
    let mut h = H::new();
    h.type_("```\n");
    h.tab().type_("a");
    assert_eq!(h.s(), "```\n    a|\n```");
    h.enter();
    assert_eq!(h.s(), "```\n    a\n    |\n```");
    h.tab();
    assert_eq!(h.s(), "```\n    a\n        |\n```");
    h.backtab();
    assert_eq!(h.s(), "```\n    a\n    |\n```");
}

#[test]
fn list_looking_lines_in_code_do_not_continue() {
    let mut h = H::new();
    h.type_("```\n- a");
    h.enter();
    assert_eq!(h.s(), "```\n- a\n|\n```");
}

#[test]
fn backspace_at_an_empty_blocks_start_removes_the_block() {
    let mut h = H::new();
    h.type_("x\n```py\n");
    assert_eq!(h.s(), "x\n```py\n|\n```");
    h.bs(1);
    assert_eq!(h.s(), "x\n|");
    assert_eq!(h.deco(), strs(&["p:x", "p:"]));
    let mut h = H::new();
    h.type_("```\n");
    h.bs(1);
    assert_eq!(h.s(), "|");
}

#[test]
fn backspace_at_a_non_empty_blocks_start_does_not_corrupt_the_fence() {
    let mut h = H::with("```\ncode\n```");
    h.m.set_cursor(4, false);
    assert_eq!(h.s(), "```\n|code\n```");
    h.bs(1);
    assert_eq!(h.text(), "```\ncode\n```");
    assert_eq!(h.deco()[1], "code:code");
    // Delete at the end of the last body line must not swallow the closing fence
    h.m.set_cursor(8, false);
    h.del();
    assert_eq!(h.text(), "```\ncode\n```");
    // nor may text above be glued onto the opening fence
    let mut h = H::with("para\n```\nx\n```");
    h.m.set_cursor(4, false);
    h.del();
    assert_eq!(h.text(), "para\n```\nx\n```");
    assert_eq!(h.deco()[0], "p:para");
}

#[test]
fn unclosed_fence_runs_to_the_end_like_commonmark() {
    let mut h = H::with("# T\n```\nx\n**b**");
    assert_eq!(h.deco(), strs(&["h1:T", "fence:", "code:x", "code:**b**"]));
    h.doc_start();
    assert_eq!(h.deco()[0], "h1:T");
}

#[test]
fn typing_a_fence_above_existing_content_does_not_wreck_it() {
    let mut h = H::with("\n**b**\n- x");
    h.doc_start();
    h.type_("```");
    // plain text while the caret is on it; the rest keeps rendering
    assert_eq!(h.deco(), strs(&["p:```", "p:b", "p:•_x"]));
    h.enter();
    assert_eq!(h.text(), "```\n\n```\n**b**\n- x");
    assert_eq!(h.deco(), strs(&["fence:", "code:", "fence:", "p:b", "p:•_x"]));
    assert_eq!(h.s(), "```\n|\n```\n**b**\n- x");
}

#[test]
fn leaving_an_unclosed_typed_fence_makes_it_a_block() {
    let mut h = H::with("\n**b**");
    h.doc_start().type_("```").down();
    assert_eq!(h.deco(), strs(&["fence:", "code:**b**"]));
    h.up();
    assert_eq!(h.deco(), strs(&["p:```", "p:b"]));
}

#[test]
fn enter_on_an_existing_blocks_fence_lines_does_not_autoclose() {
    let mut h = H::with("```\ncode\n```");
    h.doc_start().end().enter();
    assert_eq!(h.text(), "```\n\ncode\n```");
    let mut h = H::with("```\ncode\n```");
    h.enter();
    assert_eq!(h.text(), "```\ncode\n```\n");
}

#[test]
fn enter_after_a_new_fence_pairs_with_the_stolen_closer() {
    // a well-formed block below; a fence typed above it needs its own closer
    let mut h = H::with("\n```\ncode\n```");
    h.doc_start().type_("```");
    h.enter();
    assert_eq!(h.text(), "```\n\n```\n```\ncode\n```");
}

#[test]
fn deleting_the_fences_dissolves_the_block() {
    let mut h = H::with("```\nx\n```");
    h.bs(3);
    assert_eq!(h.text(), "```\nx\n");
    h.bs(1);
    assert_eq!(h.text(), "```\nx");
    h.bs(1);
    h.bs(1);
    assert_eq!(h.text(), "```");
    h.bs(3);
    assert_eq!(h.text(), "");
    assert_eq!(h.deco(), strs(&["p:"]));
}

#[test]
fn selecting_and_deleting_a_fence_line_works() {
    let mut h = H::with("a\n```\nx\n```\nb");
    h.m.set_selection(2..6);
    h.m.backspace();
    h.check();
    assert_eq!(h.text(), "a\nx\n```\nb");
}

#[test]
fn fence_inside_a_list_item() {
    let mut h = H::new();
    h.raw("- a\n  ");
    h.type_("```ts");
    h.enter();
    assert_eq!(h.s(), "- a\n  ```ts\n  |\n  ```");
    assert_eq!(h.deco(), strs(&["p:•_a", "fence:", "code:  ", "fence:"]));
    h.type_("let x");
    assert_eq!(h.deco()[2], "code:  let x");
    // below the block is a normal paragraph again
    h.doc_end().enter().type_("after **b**");
    assert_eq!(h.deco().last().unwrap(), "p:  after b", "still inside the list item, so it keeps the indent");
}

#[test]
fn adjacent_blocks_stay_separate_blocks() {
    let h = H::with("```\na\n```\n```\nb\n```");
    let ids: Vec<_> = h.m.analysis.lines.iter().map(|l| l.code.as_ref().unwrap().block).collect();
    assert_eq!(ids, vec![0, 0, 0, 10, 10, 10]);
}

#[test]
fn tilde_fences_work_and_dont_mix_with_backticks() {
    let mut h = H::new();
    h.type_("~~~");
    h.enter();
    assert_eq!(h.text(), "~~~\n\n~~~");
    let mut h = H::with("~~~\n```\nx\n~~~");
    h.doc_start().down();
    assert_eq!(h.deco(), strs(&["fence:", "code:```", "code:x", "fence:"]));
}

#[test]
fn slash_menu_inserts_a_code_block_like_typing() {
    let mut h = H::new();
    h.type_("/code");
    assert!(h.m.slash.is_some());
    h.enter();
    assert_eq!(h.s(), "```\n|\n```");
    assert_eq!(h.deco(), strs(&["fence:", "code:", "fence:"]));
    // the slash menu does not open inside code
    h.type_("/");
    assert!(h.m.slash.is_none());
}

#[test]
fn slash_menu_blocks_match_typed_markdown() {
    for (query, typed) in [("h1", "# "), ("h2", "## "), ("bullet", "- "), ("number", "1. "), ("check", "- [ ] "), ("quote", "> ")] {
        let mut a = H::new();
        a.type_(&format!("/{query}"));
        a.enter();
        let mut b = H::new();
        b.type_(typed);
        assert_eq!(a.text(), b.text(), "{query}");
        assert_eq!(a.views(), b.views(), "{query}");
    }
}

// ---- undo / redo -------------------------------------------------------------

#[test]
fn undo_redo_never_leaves_stale_decorations() {
    for s in CORPUS {
        let mut h = H::new();
        h.raw(s);
        let done_text = h.text().to_string();
        let done = h.views().to_vec();
        for _ in 0..300 {
            h.undo();
        }
        assert_eq!(h.text(), "", "{s:?}");
        assert_eq!(h.deco(), strs(&["p:"]));
        for _ in 0..300 {
            h.redo();
        }
        assert_eq!(h.text(), done_text, "{s:?}");
        h.doc_end();
        assert_eq!(h.views(), &done[..], "{s:?}");
    }
}

#[test]
fn undo_after_reveal_and_backspace_restores_a_concealed_marker() {
    let mut h = H::new();
    h.type_("# Title");
    h.m.buffer.reset("# Title");
    h.home().bs(2);
    assert_eq!(h.text(), "#Title");
    h.undo();
    assert_eq!(h.text(), "# Title");
    assert_eq!(h.deco(), strs(&["h1:Title"]));
    assert!(h.m.reveal.is_none());
}

#[test]
fn undo_of_an_autoclosed_fence_removes_the_whole_pair() {
    let mut h = H::new();
    h.type_("```py");
    h.enter();
    assert_eq!(h.text(), "```py\n\n```");
    h.undo();
    assert_eq!(h.text(), "```py");
    assert_eq!(h.deco(), strs(&["p:```py"]));
    h.redo();
    h.doc_start().down();
    assert_eq!(h.deco(), strs(&["fence:", "code:", "fence:"]));
}

#[test]
fn undo_after_enter_continuation_and_tab() {
    let mut h = H::new();
    h.type_("- a\nb");
    h.tab();
    h.undo();
    assert_eq!(h.text(), "- a\n- b");
    h.undo();
    assert_eq!(h.text(), "- a\n- ");
    h.undo();
    assert_eq!(h.text(), "- a");
}

// ---- fuzz --------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[test]
fn random_edit_sequences_keep_decorations_consistent() {
    const ALPHABET: &[char] = &['a', 'b', ' ', ' ', '#', '-', '*', '`', '`', '~', '>', '[', ']', '(', ')', '1', '.', '_', 'x', '\n', '/', '\u{e9}'];
    let mut rng = Rng(0x9E3779B97F4A7C15);
    // FUZZ_ROUNDS=20000 cargo test random_edit for a long run.
    let rounds = std::env::var("FUZZ_ROUNDS").ok().and_then(|v| v.parse().ok()).unwrap_or(400);
    for round in 0..rounds {
        let seed = CORPUS[round % CORPUS.len()];
        let mut h = H::with(seed);
        let mut log = vec![format!("start {seed:?}")];
        for _ in 0..70 {
            let op = match rng.below(22) {
                0..=5 => {
                    let ch = ALPHABET[rng.below(ALPHABET.len())];
                    h.m.replace_text(None, &ch.to_string());
                    format!("type {ch:?}")
                }
                6 | 7 => {
                    h.m.backspace();
                    "backspace".into()
                }
                8 => {
                    h.m.delete();
                    "delete".into()
                }
                9 | 10 => {
                    h.m.enter();
                    "enter".into()
                }
                11 => {
                    h.m.tab();
                    "tab".into()
                }
                12 => {
                    h.m.backtab();
                    "backtab".into()
                }
                13 => {
                    let sel = rng.below(4) == 0;
                    h.m.move_left(sel);
                    format!("left select={sel}")
                }
                14 => {
                    let sel = rng.below(4) == 0;
                    h.m.move_right(sel);
                    format!("right select={sel}")
                }
                15 => {
                    let sel = rng.below(4) == 0;
                    h.m.move_vertical(-1, sel);
                    format!("up select={sel}")
                }
                16 => {
                    let sel = rng.below(4) == 0;
                    h.m.move_vertical(1, sel);
                    format!("down select={sel}")
                }
                17 => {
                    h.m.undo();
                    "undo".into()
                }
                18 => {
                    h.m.redo();
                    "redo".into()
                }
                19 => {
                    let p = CORPUS[rng.below(CORPUS.len())];
                    h.m.insert(p);
                    format!("paste {p:?}")
                }
                20 => {
                    h.m.toggle_check();
                    "toggle".into()
                }
                _ => {
                    let at = rng.below(h.text().len() + 1);
                    let at = (0..=at).rev().find(|i| h.text().is_char_boundary(*i)).unwrap();
                    let sel = rng.below(3) == 0;
                    h.m.set_cursor(at, sel);
                    format!("click {at} select={sel}")
                }
            };
            log.push(format!("{op} -> {}", h.s()));
            if let Err(e) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.check())) {
                eprintln!("fuzz failure after:\n  {}", log.join("\n  "));
                std::panic::resume_unwind(e);
            }
        }
    }
}

#[test]
fn random_sequences_only_ever_produce_parseable_line_structure() {
    // Whatever the edits, every line has exactly one view and views cover the text.
    let mut rng = Rng(42);
    let mut h = H::new();
    for _ in 0..2000 {
        let ch = ['`', '\n', '#', ' ', '-', 'a', '*'][rng.below(7)];
        if rng.below(5) == 0 {
            h.m.backspace();
        } else if ch == '\n' {
            h.m.enter();
        } else {
            h.m.replace_text(None, &ch.to_string());
        }
        h.check();
    }
}
