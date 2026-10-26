//! The bottom toolbar: counts, file name, and the Search / Export / Settings buttons.

use crate::stats;
use crate::theme::Theme;
use crate::workspace::Workspace;
use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, SharedString, StatefulInteractiveElement,
    Styled, div, px,
};

pub const HEIGHT: f32 = 44.0;

fn button(id: &'static str, label: &'static str, theme: Theme) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px(px(8.))
        .py(px(3.))
        .rounded(px(5.))
        .cursor_pointer()
        .text_color(theme.muted)
        .hover(move |d| d.bg(theme.hover).text_color(theme.fg))
        .child(label)
}

pub fn render(ws: &Workspace, cx: &mut Context<Workspace>) -> impl IntoElement {
    let theme = *cx.global::<Theme>();
    let editor = ws.editor.read(cx);
    let text = editor.plain_text();
    let words = stats::word_count(&text);
    let chars = stats::char_count(&text);
    let name: SharedString = editor.file_name().into();
    let dirty = editor.is_dirty();
    let notice = ws.notice.clone();

    let counts = format!(
        "{} {}  ·  {} {}",
        fmt_num(words),
        if words == 1 { "word" } else { "words" },
        fmt_num(chars),
        if chars == 1 { "character" } else { "characters" }
    );

    div()
        .h(px(HEIGHT))
        .absolute()
        .bottom_0()
        .left_0()
        .w_full()
        .flex()
        .items_center()
        .px(px(24.))
        .font_family(".SystemUIFont")
        .text_size(px(12.))
        .text_color(theme.muted)
        .child(div().flex_1().child(counts))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(match notice {
                    Some(n) => div().text_color(theme.accent).child(n),
                    None => div().text_color(theme.fg.opacity(0.75)).child(name),
                })
                .child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .when_dirty(dirty, theme),
                ),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .justify_end()
                .gap(px(2.))
                .child(button("tb-search", "Search", theme).on_click(cx.listener(|ws, _, window, cx| ws.toggle_find(window, cx))))
                .child(button("tb-export", "Export", theme).on_click(cx.listener(|ws, _, window, cx| ws.toggle_export_menu(window, cx))))
                .child(button("tb-settings", "Settings", theme).on_click(cx.listener(|ws, _, window, cx| ws.toggle_settings(window, cx)))),
        )
}

trait DirtyDot {
    fn when_dirty(self, dirty: bool, theme: Theme) -> Self;
}

impl DirtyDot for gpui::Div {
    fn when_dirty(self, dirty: bool, theme: Theme) -> Self {
        if dirty { self.bg(theme.accent) } else { self }
    }
}

/// 12345 → "12,345"
pub fn fmt_num(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::fmt_num;

    #[test]
    fn formats_thousands() {
        assert_eq!(fmt_num(0), "0");
        assert_eq!(fmt_num(999), "999");
        assert_eq!(fmt_num(1000), "1,000");
        assert_eq!(fmt_num(1234567), "1,234,567");
    }
}
