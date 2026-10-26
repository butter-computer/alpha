//! Find-in-document bar (Cmd+F).

use crate::actions::*;
use crate::editor::Editor;
use crate::input::{InputEvent, LineInput};
use crate::theme::Theme;
use gpui::{
    AppContext,    Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Subscription, Window, div, px,
};

pub struct FindBar {
    input: Entity<LineInput>,
    editor: Entity<Editor>,
    _subs: [Subscription; 2],
}

impl Focusable for FindBar {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }
}

impl FindBar {
    pub fn new(editor: Entity<Editor>, initial: Option<String>, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| LineInput::new("Find", cx));
        let ed = editor.clone();
        let s1 = cx.subscribe(&input, move |this, input, _: &InputEvent, cx| {
            let q = input.read(cx).content.clone();
            this.editor.update(cx, |e, cx| e.set_find_query(&q, cx));
        });
        let s2 = cx.observe(&ed, |_, _, cx| cx.notify());
        let this = Self { input, editor, _subs: [s1, s2] };
        if let Some(text) = initial {
            this.input.update(cx, |i, cx| i.set_content(&text, cx));
        }
        this
    }

    pub fn select_all_text(&self, cx: &mut Context<Self>) {
        let q = self.input.read(cx).content.clone();
        self.input.update(cx, |i, cx| i.set_content(&q, cx));
    }

    fn confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |e, cx| e.find_step(true, cx));
    }

    fn confirm_reverse(&mut self, _: &ConfirmReverse, _: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |e, cx| e.find_step(false, cx));
    }
}

impl Render for FindBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.global::<Theme>();
        let ed = self.editor.read(cx);
        let status = if ed.find.query.is_empty() {
            String::new()
        } else if ed.find.matches.is_empty() {
            "No results".to_string()
        } else {
            format!("{} of {}", ed.find.current.map(|i| i + 1).unwrap_or(0), ed.find.matches.len())
        };
        div()
            .absolute()
            .top(px(14.))
            .right(px(20.))
            .w(px(280.))
            .occlude()
            .on_action(cx.listener(Self::confirm))
            .on_action(cx.listener(Self::confirm_reverse))
            .flex()
            .items_center()
            .gap(px(10.))
            .px(px(12.))
            .py(px(7.))
            .bg(theme.panel)
            .border_1()
            .border_color(theme.hairline)
            .rounded(px(8.))
            .shadow_md()
            .font_family(".SystemUIFont")
            .text_size(px(13.))
            .text_color(theme.fg)
            .child(div().flex_1().child(self.input.clone()))
            .child(div().text_size(px(11.)).text_color(theme.muted).child(status))
    }
}
