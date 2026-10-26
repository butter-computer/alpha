//! The root view: owns the editor, overlays (palette, find, settings, export
//! menu), and file operations.

use crate::actions::*;
use crate::editor::Editor;
use crate::export;
use crate::findbar::FindBar;
use crate::input::{InputEvent, LineInput};
use crate::palette::{Command, Palette, PaletteEvent};
use crate::settings::{FontChoice, Settings, ThemeMode};
use crate::sound::{self, Cue};
use crate::theme::Theme;
use crate::toolbar;
use gpui::{
    AppContext, Context, Div, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, PathPromptOptions, PromptLevel, Render, SharedString, StatefulInteractiveElement,
    Styled, Subscription, Task, Window, WindowAppearance, div, prelude::*, px,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Overlay panels sit this far above the window bottom.
const PANEL_BOTTOM: f32 = toolbar::HEIGHT + 10.0;

pub struct Workspace {
    pub editor: Entity<Editor>,
    palette: Option<Entity<Palette>>,
    find: Option<Entity<FindBar>>,
    settings_open: bool,
    export_open: bool,
    /// The toolbar name being edited in place.
    pub renaming: Option<Entity<LineInput>>,
    _rename_subs: Vec<Subscription>,
    pub notice: Option<SharedString>,
    notice_gen: u64,
    last_title: String,
    _subs: Vec<Subscription>,
}

impl Workspace {
    pub fn new(open: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (text, path) = match open.and_then(|p| std::fs::read_to_string(&p).ok().map(|t| (t, p))) {
            Some((t, p)) => (t, Some(p)),
            None => (String::new(), None),
        };
        let editor = cx.new(|cx| Editor::new(text, path, cx));
        let mut subs = vec![cx.observe(&editor, |_, _, cx| cx.notify())];
        subs.push(cx.observe_window_appearance(window, |this, window, cx| {
            this.apply_theme(window, cx);
        }));
        let ws = cx.entity();
        window.on_window_should_close(cx, move |window, cx| {
            if !ws.read(cx).editor.read(cx).is_dirty() {
                return true;
            }
            ws.update(cx, |t, cx| t.guard_dirty(window, cx, |_, _, cx| cx.quit()));
            false
        });
        let mut this = Self {
            editor,
            palette: None,
            find: None,
            settings_open: false,
            export_open: false,
            renaming: None,
            _rename_subs: Vec::new(),
            notice: None,
            notice_gen: 0,
            last_title: String::new(),
            _subs: subs,
        };
        this.apply_theme(window, cx);
        this
    }

    // ---- look ---------------------------------------------------------------

    pub fn apply_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dark = match cx.global::<Settings>().theme {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => matches!(window.appearance(), WindowAppearance::Dark | WindowAppearance::VibrantDark),
        };
        let theme = if dark { Theme::dark() } else { Theme::light() };
        cx.set_global(theme);
        crate::edge_blur::configure(&theme);
        window.refresh();
        cx.notify();
    }

    fn update_settings(&mut self, window: &mut Window, cx: &mut Context<Self>, f: impl FnOnce(&mut Settings)) {
        cx.update_global::<Settings, _>(|s, _| {
            f(s);
            s.sanitize();
            s.save();
        });
        self.apply_theme(window, cx);
    }

    // ---- overlays -----------------------------------------------------------

    fn focus_editor(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.editor.focus_handle(cx));
    }

    /// Closes every overlay; returns whether anything was open.
    fn close_overlays(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let any = self.palette.is_some() || self.find.is_some() || self.settings_open || self.export_open;
        if self.find.take().is_some() {
            self.editor.update(cx, |e, cx| e.clear_find(cx));
        }
        self.palette = None;
        self.settings_open = false;
        self.export_open = false;
        if any {
            self.focus_editor(window, cx);
            cx.notify();
        }
        any
    }

    pub fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.is_some() {
            self.close_overlays(window, cx);
            return;
        }
        self.close_overlays(window, cx);
        let palette = cx.new(Palette::new);
        self._subs.push(cx.subscribe_in(&palette, window, |this, _, ev: &PaletteEvent, window, cx| match ev {
            PaletteEvent::Run(cmd) => {
                this.close_overlays(window, cx);
                this.run_command(*cmd, window, cx);
            }
            PaletteEvent::Dismiss => {
                if this.palette.is_some() {
                    this.close_overlays(window, cx);
                }
            }
        }));
        window.focus(&palette.focus_handle(cx));
        self.palette = Some(palette);
        cx.notify();
    }

    pub fn toggle_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(f) = &self.find {
            let f = f.clone();
            window.focus(&f.focus_handle(cx));
            f.update(cx, |f, cx| f.select_all_text(cx));
            return;
        }
        self.close_overlays(window, cx);
        let initial = self
            .editor
            .read(cx)
            .selected_text()
            .filter(|t| !t.contains('\n') && t.len() < 200)
            .map(str::to_string);
        let editor = self.editor.clone();
        let bar = cx.new(|cx| FindBar::new(editor, initial, cx));
        window.focus(&bar.focus_handle(cx));
        self.find = Some(bar);
        cx.notify();
    }

    pub fn toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let open = !self.settings_open;
        self.close_overlays(window, cx);
        self.settings_open = open;
        cx.notify();
    }

    pub fn toggle_export_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let open = !self.export_open;
        self.close_overlays(window, cx);
        self.export_open = open;
        cx.notify();
    }

    // ---- commands -----------------------------------------------------------

    pub fn run_command(&mut self, cmd: Command, window: &mut Window, cx: &mut Context<Self>) {
        match cmd {
            Command::New => self.new_doc(window, cx),
            Command::Open => self.open(window, cx),
            Command::Save => self.save(false, window, cx).detach(),
            Command::SaveAs => self.save(true, window, cx).detach(),
            Command::ExportMarkdown => self.export(false, window, cx),
            Command::ExportHtml => self.export(true, window, cx),
            Command::Find => self.toggle_find(window, cx),
            Command::ToggleTheme => {
                let dark = cx.global::<Theme>().dark;
                self.update_settings(window, cx, |s| s.theme = if dark { ThemeMode::Light } else { ThemeMode::Dark });
            }
            Command::IncreaseFont => self.update_settings(window, cx, |s| s.adjust_font_size(1.0)),
            Command::DecreaseFont => self.update_settings(window, cx, |s| s.adjust_font_size(-1.0)),
            Command::Settings => self.toggle_settings(window, cx),
            Command::Rename => self.start_rename(window, cx),
        }
    }

    fn notify_msg(&mut self, msg: String, cx: &mut Context<Self>) {
        self.notice = Some(msg.into());
        self.notice_gen += 1;
        let gen_ = self.notice_gen;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(4)).await;
            this.update(cx, |t, cx| {
                if t.notice_gen == gen_ {
                    t.notice = None;
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    // ---- files --------------------------------------------------------------

    /// Runs `then` immediately if there are no unsaved changes, otherwise asks
    /// the user whether to save first.
    fn guard_dirty(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) {
        if !self.editor.read(cx).is_dirty() {
            then(self, window, cx);
            return;
        }
        let name = self.editor.read(cx).file_name();
        let rx = window.prompt(
            PromptLevel::Warning,
            &format!("Do you want to save the changes to “{name}”?"),
            Some("Your changes will be lost if you don’t save them."),
            &["Save", "Don’t Save", "Cancel"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            let Ok(choice) = rx.await else { return };
            let proceed = match choice {
                0 => match this.update_in(cx, |t, w, c| t.save(false, w, c)) {
                    Ok(task) => task.await,
                    Err(_) => false,
                },
                1 => true,
                _ => false,
            };
            if proceed {
                this.update_in(cx, |t, w, c| then(t, w, c)).ok();
            }
        })
        .detach();
    }

    fn new_doc(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.guard_dirty(window, cx, |this, window, cx| {
            this.editor.update(cx, |e, cx| e.load(String::new(), None, cx));
            this.focus_editor(window, cx);
        });
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.guard_dirty(window, cx, |_, window, cx| {
            let rx = cx.prompt_for_paths(PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some("Open".into()),
            });
            cx.spawn_in(window, async move |this, cx| {
                let Ok(Ok(Some(paths))) = rx.await else { return };
                if let Some(p) = paths.into_iter().next() {
                    this.update_in(cx, |t, w, c| t.open_path(p, w, c)).ok();
                }
            })
            .detach();
        });
    }

    pub fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                self.editor.update(cx, |e, cx| e.load(text, Some(path), cx));
                self.focus_editor(window, cx);
            }
            Err(e) => self.notify_msg(format!("Couldn’t open file: {e}"), cx),
        }
    }

    fn default_dir(&self, cx: &Context<Self>) -> PathBuf {
        self.editor
            .read(cx)
            .path
            .as_ref()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .or_else(dirs::document_dir)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn stem(&self, cx: &Context<Self>) -> String {
        self.editor.read(cx).stem()
    }

    // ---- rename -------------------------------------------------------------

    /// Turns the toolbar name into an input holding the name, all selected.
    pub fn start_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.renaming.is_some() {
            return;
        }
        self.close_overlays(window, cx);
        let stem = self.stem(cx);
        let input = cx.new(|cx| LineInput::new("Untitled", cx));
        input.update(cx, |i, cx| i.set_content(&stem, cx));
        let handle = input.focus_handle(cx);
        window.focus(&handle);
        self._rename_subs = vec![
            cx.subscribe(&input, |_, _, _: &InputEvent, cx| cx.notify()),
            // Clicking away keeps the new name, as in Finder.
            cx.on_blur(&handle, window, |this, _, cx| this.finish_rename(true, cx)),
        ];
        self.renaming = Some(input);
        cx.notify();
    }

    /// Ends a rename, applying it if `keep`: an unsaved document takes the name,
    /// a saved one has its file renamed next to itself, keeping the extension.
    pub fn finish_rename(&mut self, keep: bool, cx: &mut Context<Self>) {
        let Some(input) = self.renaming.take() else { return };
        self._rename_subs.clear();
        cx.notify();
        let Some(name) = keep.then(|| clean_name(&input.read(cx).content)).flatten() else { return };
        let Some(old) = self.editor.read(cx).path.clone() else {
            self.editor.update(cx, |e, cx| {
                e.title = (name != "Untitled").then_some(name);
                cx.notify();
            });
            return;
        };
        let ext = old.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "md".into());
        let new = old.with_file_name(format!("{name}.{ext}"));
        if new == old {
            return;
        }
        if new.exists() && !same_file(&old, &new) {
            self.notify_msg(format!("“{name}.{ext}” already exists"), cx);
            return;
        }
        match std::fs::rename(&old, &new) {
            Ok(()) => self.editor.update(cx, |e, cx| {
                e.path = Some(new);
                cx.notify();
            }),
            Err(e) => self.notify_msg(format!("Couldn’t rename: {e}"), cx),
        }
    }

    /// Enter in the name field: keep the name and go back to writing.
    pub fn confirm_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_rename(true, cx);
        self.focus_editor(window, cx);
    }

    /// Escape in the name field: drop the edit and go back to writing.
    pub fn cancel_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_rename(false, cx);
        self.focus_editor(window, cx);
    }

    /// Saves to the current path (prompting if there is none or `as_new`).
    /// Resolves to whether the document ended up saved.
    fn save(&mut self, as_new: bool, window: &mut Window, cx: &mut Context<Self>) -> Task<bool> {
        let (text, revision, path) = {
            let e = self.editor.read(cx);
            (e.text().to_string(), e.buffer().revision(), e.path.clone())
        };
        let path = if as_new { None } else { path };
        let rx = path
            .is_none()
            .then(|| cx.prompt_for_new_path(&self.default_dir(cx), Some(&format!("{}.md", self.stem(cx)))));
        cx.spawn_in(window, async move |this, cx| {
            let path = match (path, rx) {
                (Some(p), _) => p,
                (None, Some(rx)) => match rx.await {
                    Ok(Ok(Some(p))) => with_extension(p, "md"),
                    _ => return false,
                },
                _ => return false,
            };
            match std::fs::write(&path, text) {
                Ok(()) => {
                    this.update(cx, |t, cx| {
                        t.editor.update(cx, |e, cx| e.mark_saved(path, revision, cx));
                        sound::play(Cue::Saved, cx);
                    })
                    .ok();
                    true
                }
                Err(e) => {
                    this.update(cx, |t, cx| t.notify_msg(format!("Couldn’t save: {e}"), cx)).ok();
                    false
                }
            }
        })
    }

    fn export(&mut self, html: bool, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.editor.read(cx).text().to_string();
        let stem = self.stem(cx);
        let ext = if html { "html" } else { "md" };
        let rx = cx.prompt_for_new_path(&self.default_dir(cx), Some(&format!("{stem}.{ext}")));
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(path))) = rx.await else { return };
            let path = with_extension(path, ext);
            let content = if html { export::markdown_to_html(&text, &stem) } else { text };
            let written = std::fs::write(&path, content);
            let msg = match &written {
                Ok(()) => format!("Exported {}", path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
                Err(e) => format!("Couldn’t export: {e}"),
            };
            this.update(cx, |t, cx| {
                if written.is_ok() {
                    sound::play(Cue::Exported, cx);
                }
                t.notify_msg(msg, cx)
            })
            .ok();
        })
        .detach();
    }

    fn sync_title(&mut self, window: &mut Window, cx: &Context<Self>) {
        let title = format!("{} — alpha", self.editor.read(cx).file_name());
        if title != self.last_title {
            window.set_window_title(&title);
            self.last_title = title;
        }
    }

    // ---- panels -------------------------------------------------------------

    fn settings_panel(&self, theme: Theme, settings: &Settings, cx: &mut Context<Self>) -> impl IntoElement {
        let seg = |id: &'static str, label: &'static str, active: bool| {
            div()
                .id(id)
                .flex_1()
                .flex()
                .justify_center()
                .py(px(4.))
                .rounded(px(5.))
                .cursor_pointer()
                .text_color(if active { theme.fg } else { theme.muted })
                .when(active, |d| d.bg(theme.hover))
                .hover(move |d| d.text_color(theme.fg))
                .child(label)
        };
        let row = |label: &'static str| div().flex().flex_col().gap(px(6.)).child(div().text_size(px(11.)).text_color(theme.muted).child(label));
        let t = settings.theme;
        let f = settings.font;
        let on = settings.sounds;
        div()
            .absolute()
            .bottom(px(PANEL_BOTTOM))
            .right(px(16.))
            .w(px(250.))
            .occlude()
            .on_mouse_down_out(cx.listener(|ws, _, window, cx| {
                if ws.settings_open {
                    ws.close_overlays(window, cx);
                }
            }))
            .flex()
            .flex_col()
            .gap(px(14.))
            .p(px(16.))
            .bg(theme.panel)
            .border_1()
            .border_color(theme.hairline)
            .rounded(px(10.))
            .shadow_lg()
            .font_family(".SystemUIFont")
            .text_size(px(13.))
            .text_color(theme.fg)
            .child(
                row("Theme").child(
                    div()
                        .flex()
                        .gap(px(2.))
                        .child(seg("th-sys", "System", t == ThemeMode::System).on_click(cx.listener(|ws, _, w, cx| ws.update_settings(w, cx, |s| s.theme = ThemeMode::System))))
                        .child(seg("th-light", "Light", t == ThemeMode::Light).on_click(cx.listener(|ws, _, w, cx| ws.update_settings(w, cx, |s| s.theme = ThemeMode::Light))))
                        .child(seg("th-dark", "Dark", t == ThemeMode::Dark).on_click(cx.listener(|ws, _, w, cx| ws.update_settings(w, cx, |s| s.theme = ThemeMode::Dark)))),
                ),
            )
            .child(
                row("Font size").child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(2.))
                        .child(seg("fs-dec", "−", false).on_click(cx.listener(|ws, _, w, cx| ws.update_settings(w, cx, |s| s.adjust_font_size(-1.0)))))
                        .child(div().flex_1().flex().justify_center().child(format!("{}", settings.font_size.round() as i32)))
                        .child(seg("fs-inc", "+", false).on_click(cx.listener(|ws, _, w, cx| ws.update_settings(w, cx, |s| s.adjust_font_size(1.0))))),
                ),
            )
            .child(
                row("Font").child(
                    div()
                        .flex()
                        .gap(px(2.))
                        .child(seg("ff-serif", "Serif", f == FontChoice::Serif).on_click(cx.listener(|ws, _, w, cx| ws.update_settings(w, cx, |s| s.font = FontChoice::Serif))))
                        .child(seg("ff-sans", "Sans", f == FontChoice::Sans).on_click(cx.listener(|ws, _, w, cx| ws.update_settings(w, cx, |s| s.font = FontChoice::Sans))))
                        .child(seg("ff-mono", "Mono", f == FontChoice::Mono).on_click(cx.listener(|ws, _, w, cx| ws.update_settings(w, cx, |s| s.font = FontChoice::Mono)))),
                ),
            )
            .child(
                row("Sounds").child(
                    div()
                        .flex()
                        .gap(px(2.))
                        .child(seg("snd-on", "On", on).on_click(cx.listener(|ws, _, w, cx| {
                            ws.update_settings(w, cx, |s| s.sounds = true);
                            sound::play(Cue::Check, cx);
                        })))
                        .child(seg("snd-off", "Off", !on).on_click(cx.listener(|ws, _, w, cx| ws.update_settings(w, cx, |s| s.sounds = false)))),
                ),
            )
    }

    fn export_menu(&self, theme: Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let item = |id: &'static str, label: &'static str| {
            div()
                .id(id)
                .px(px(10.))
                .py(px(6.))
                .rounded(px(5.))
                .cursor_pointer()
                .hover(move |d| d.bg(theme.hover))
                .child(label)
        };
        div()
            .absolute()
            .bottom(px(PANEL_BOTTOM))
            .right(px(96.))
            .w(px(190.))
            .occlude()
            .on_mouse_down_out(cx.listener(|ws, _, window, cx| {
                if ws.export_open {
                    ws.close_overlays(window, cx);
                }
            }))
            .p(px(5.))
            .bg(theme.panel)
            .border_1()
            .border_color(theme.hairline)
            .rounded(px(10.))
            .shadow_lg()
            .font_family(".SystemUIFont")
            .text_size(px(13.))
            .text_color(theme.fg)
            .child(item("ex-md", "Markdown (.md)").on_click(cx.listener(|ws, _, w, cx| {
                ws.close_overlays(w, cx);
                ws.run_command(Command::ExportMarkdown, w, cx);
            })))
            .child(item("ex-html", "HTML (.html)").on_click(cx.listener(|ws, _, w, cx| {
                ws.close_overlays(w, cx);
                ws.run_command(Command::ExportHtml, w, cx);
            })))
    }
}

fn with_extension(mut p: PathBuf, ext: &str) -> PathBuf {
    if p.extension().is_none() {
        p.set_extension(ext);
    }
    p
}

impl Focusable for Workspace {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        self.editor.read(cx).focus_handle.clone()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_title(window, cx);
        let theme = *cx.global::<Theme>();
        let settings = cx.global::<Settings>().clone();
        let root: Div = div();
        root.key_context("Workspace")
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(theme.bg)
            .text_color(theme.fg)
            .font_family(settings.font.family())
            .text_size(px(settings.font_size))
            .on_action(cx.listener(|ws, _: &NewDoc, w, cx| ws.run_command(Command::New, w, cx)))
            .on_action(cx.listener(|ws, _: &OpenDoc, w, cx| ws.run_command(Command::Open, w, cx)))
            .on_action(cx.listener(|ws, _: &Save, w, cx| ws.run_command(Command::Save, w, cx)))
            .on_action(cx.listener(|ws, _: &SaveAs, w, cx| ws.run_command(Command::SaveAs, w, cx)))
            .on_action(cx.listener(|ws, _: &ExportMarkdown, w, cx| ws.run_command(Command::ExportMarkdown, w, cx)))
            .on_action(cx.listener(|ws, _: &ExportHtml, w, cx| ws.run_command(Command::ExportHtml, w, cx)))
            .on_action(cx.listener(|ws, _: &ToggleFind, w, cx| ws.toggle_find(w, cx)))
            .on_action(cx.listener(|ws, _: &TogglePalette, w, cx| ws.toggle_palette(w, cx)))
            .on_action(cx.listener(|ws, _: &ToggleSettings, w, cx| ws.toggle_settings(w, cx)))
            .on_action(cx.listener(|ws, _: &ToggleExportMenu, w, cx| ws.toggle_export_menu(w, cx)))
            .on_action(cx.listener(|ws, _: &ToggleTheme, w, cx| ws.run_command(Command::ToggleTheme, w, cx)))
            .on_action(cx.listener(|ws, _: &IncreaseFont, w, cx| ws.run_command(Command::IncreaseFont, w, cx)))
            .on_action(cx.listener(|ws, _: &DecreaseFont, w, cx| ws.run_command(Command::DecreaseFont, w, cx)))
            .on_action(cx.listener(|ws, _: &Quit, w, cx| ws.guard_dirty(w, cx, |_, _, cx| cx.quit())))
            .on_action(cx.listener(|ws, _: &Cancel, w, cx| {
                ws.close_overlays(w, cx);
            }))
            .child(div().absolute().top_0().left_0().size_full().child(self.editor.clone()))
            // Everything above this marker is edge-blurred; the toolbar and popups stay sharp.
            .child(crate::edge_blur::BlurSplit)
            .child(toolbar::render(self, cx))
            .children(self.palette.clone())
            .children(self.find.clone())
            .when(self.settings_open, |d| d.child(self.settings_panel(theme, &settings, cx)))
            .when(self.export_open, |d| d.child(self.export_menu(theme, cx)))
    }
}

/// A typed name made safe to use as a file stem: trimmed, no path separators,
/// not hidden, without a markdown extension the user typed themselves.
/// `None` when nothing usable is left.
fn clean_name(raw: &str) -> Option<String> {
    let mut name = raw.trim().replace(['/', ':'], "-");
    for ext in [".md", ".markdown"] {
        if name.len() > ext.len() && name.to_lowercase().ends_with(ext) {
            name.truncate(name.len() - ext.len());
        }
    }
    let name = name.trim().trim_start_matches('.').trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Whether two paths are the same file (a case-only rename on a case-insensitive disk).
fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::clean_name;

    #[test]
    fn names_are_cleaned_for_the_file_system() {
        assert_eq!(clean_name("  Notes  ").as_deref(), Some("Notes"));
        assert_eq!(clean_name("draft.md").as_deref(), Some("draft"));
        assert_eq!(clean_name("Draft.MD").as_deref(), Some("Draft"));
        assert_eq!(clean_name("a/b:c").as_deref(), Some("a-b-c"));
        assert_eq!(clean_name(".hidden").as_deref(), Some("hidden"));
        assert_eq!(clean_name("v1.2 notes").as_deref(), Some("v1.2 notes"));
        assert_eq!(clean_name("   "), None);
        assert_eq!(clean_name("..."), None);
    }
}
