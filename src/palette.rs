//! Cmd+K command palette with fuzzy filtering.

use crate::actions::*;
use crate::fuzzy;
use crate::input::{InputEvent, LineInput};
use crate::theme::Theme;
use gpui::{
    Context, Entity, EventEmitter, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::*,
    px,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    New,
    Open,
    Save,
    SaveAs,
    Rename,
    ExportMarkdown,
    ExportHtml,
    Find,
    ToggleTheme,
    IncreaseFont,
    DecreaseFont,
    Settings,
}

pub struct Entry {
    pub command: Command,
    pub label: &'static str,
    pub shortcut: &'static str,
}

pub const COMMANDS: &[Entry] = &[
    Entry { command: Command::New, label: "New", shortcut: "⌘N" },
    Entry { command: Command::Open, label: "Open…", shortcut: "⌘O" },
    Entry { command: Command::Save, label: "Save", shortcut: "⌘S" },
    Entry { command: Command::SaveAs, label: "Save As…", shortcut: "⇧⌘S" },
    Entry { command: Command::Rename, label: "Rename…", shortcut: "" },
    Entry { command: Command::ExportMarkdown, label: "Export Markdown", shortcut: "" },
    Entry { command: Command::ExportHtml, label: "Export HTML", shortcut: "" },
    Entry { command: Command::Find, label: "Find", shortcut: "⌘F" },
    Entry { command: Command::ToggleTheme, label: "Toggle Theme", shortcut: "" },
    Entry { command: Command::IncreaseFont, label: "Increase Font Size", shortcut: "⌘=" },
    Entry { command: Command::DecreaseFont, label: "Decrease Font Size", shortcut: "⌘−" },
    Entry { command: Command::Settings, label: "Settings", shortcut: "⌘," },
];

pub fn filter(query: &str) -> Vec<&'static Entry> {
    fuzzy::filter(query, COMMANDS, |e| e.label).into_iter().collect()
}

pub enum PaletteEvent {
    Run(Command),
    Dismiss,
}

pub struct Palette {
    input: Entity<LineInput>,
    selected: usize,
    _sub: Subscription,
}

impl EventEmitter<PaletteEvent> for Palette {}

impl Focusable for Palette {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.input.read(cx).focus_handle(cx)
    }
}

impl Palette {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| LineInput::new("Type a command…", cx));
        let sub = cx.subscribe(&input, |this, _, _: &InputEvent, cx| {
            this.selected = 0;
            cx.notify();
        });
        Self { input, selected: 0, _sub: sub }
    }

    fn entries(&self, cx: &gpui::App) -> Vec<&'static Entry> {
        filter(&self.input.read(cx).content)
    }

    fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        let n = self.entries(cx).len();
        if n > 0 {
            self.selected = (self.selected + n - 1) % n;
        }
        cx.notify();
    }

    fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        let n = self.entries(cx).len();
        if n > 0 {
            self.selected = (self.selected + 1) % n;
        }
        cx.notify();
    }

    fn confirm(&mut self, _: &Confirm, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(e) = self.entries(cx).get(self.selected) {
            cx.emit(PaletteEvent::Run(e.command));
        }
    }
}

impl Render for Palette {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *cx.global::<Theme>();
        let entries = self.entries(cx);
        let selected = self.selected.min(entries.len().saturating_sub(1));
        div()
            .absolute()
            .top(px(84.))
            .w_full()
            .flex()
            .justify_center()
            .child(
                div()
                    .on_action(cx.listener(Self::up))
                    .on_action(cx.listener(Self::down))
                    .on_action(cx.listener(Self::confirm))
                    .on_mouse_down_out(cx.listener(|_, _, _, cx| cx.emit(PaletteEvent::Dismiss)))
                    .occlude()
                    .w(px(500.))
                    .flex()
                    .flex_col()
                    .bg(theme.panel)
                    .border_1()
                    .border_color(theme.hairline)
                    .rounded(px(10.))
                    .shadow_lg()
                    .overflow_hidden()
                    .font_family(".SystemUIFont")
                    .text_size(px(14.))
                    .text_color(theme.fg)
                    .child(div().px(px(16.)).py(px(12.)).child(self.input.clone()))
                    .when(!entries.is_empty(), |d| {
                        d.child(
                            div()
                                .border_t_1()
                                .border_color(theme.hairline)
                                .p(px(6.))
                                .flex()
                                .flex_col()
                                .children(entries.iter().enumerate().map(|(i, e)| {
                                    let cmd = e.command;
                                    div()
                                        .id(("cmd", i))
                                        .flex()
                                        .justify_between()
                                        .px(px(10.))
                                        .py(px(6.))
                                        .rounded(px(6.))
                                        .cursor_pointer()
                                        .when(i == selected, |d| d.bg(theme.hover))
                                        .hover(|d| d.bg(theme.hover))
                                        .on_click(cx.listener(move |_, _, _, cx| cx.emit(PaletteEvent::Run(cmd))))
                                        .child(e.label)
                                        .child(div().text_color(theme.muted).text_size(px(12.)).child(e.shortcut))
                                })),
                        )
                    })
                    .when(entries.is_empty(), |d| {
                        d.child(
                            div()
                                .border_t_1()
                                .border_color(theme.hairline)
                                .px(px(16.))
                                .py(px(12.))
                                .text_color(theme.muted)
                                .child("No matching commands"),
                        )
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_lists_all_commands() {
        assert_eq!(filter("").len(), COMMANDS.len());
    }

    #[test]
    fn fuzzy_finds_commands() {
        assert_eq!(filter("exph")[0].command, Command::ExportHtml);
        assert_eq!(filter("sv")[0].command, Command::Save);
        assert_eq!(filter("theme")[0].command, Command::ToggleTheme);
        assert!(filter("qqq").is_empty());
    }

    #[test]
    fn required_commands_present() {
        for label in [
            "New", "Open…", "Save", "Save As…", "Rename…", "Export Markdown", "Export HTML", "Find",
            "Toggle Theme", "Increase Font Size", "Decrease Font Size", "Settings",
        ] {
            assert!(COMMANDS.iter().any(|e| e.label == label), "{label}");
        }
    }
}
