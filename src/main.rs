mod actions;
mod editor;
mod export;
mod findbar;
mod fonts;
mod fuzzy;
mod input;
mod markdown;
mod palette;
mod placeholders;
mod search;
mod settings;
mod stats;
mod theme;
mod toolbar;
mod workspace;

use gpui::{
    App, AppContext, Application, Bounds, Focusable, Menu, MenuItem, TitlebarOptions, WindowBounds,
    WindowOptions, point, px, size,
};
use workspace::Workspace;

fn menus() -> Vec<Menu> {
    use actions::*;
    vec![
        Menu { name: "alpha".into(), items: vec![MenuItem::action("Settings…", ToggleSettings), MenuItem::separator(), MenuItem::action("Quit alpha", Quit)] },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New", NewDoc),
                MenuItem::action("Open…", OpenDoc),
                MenuItem::separator(),
                MenuItem::action("Save", Save),
                MenuItem::action("Save As…", SaveAs),
                MenuItem::separator(),
                MenuItem::action("Export Markdown…", ExportMarkdown),
                MenuItem::action("Export HTML…", ExportHtml),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", Undo),
                MenuItem::action("Redo", Redo),
                MenuItem::separator(),
                MenuItem::action("Cut", Cut),
                MenuItem::action("Copy", Copy),
                MenuItem::action("Paste", Paste),
                MenuItem::action("Select All", SelectAll),
                MenuItem::separator(),
                MenuItem::action("Find", ToggleFind),
                MenuItem::action("Command Palette", TogglePalette),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Theme", ToggleTheme),
                MenuItem::action("Increase Font Size", IncreaseFont),
                MenuItem::action("Decrease Font Size", DecreaseFont),
            ],
        },
    ]
}

fn main() {
    let open = std::env::args().nth(1).map(std::path::PathBuf::from);
    Application::new().run(move |cx: &mut App| {
        cx.text_system().add_fonts(fonts::bundled()).expect("failed to register bundled fonts");
        cx.set_global(settings::Settings::load());
        cx.set_global(theme::Theme::light());
        actions::bind_keys(cx);
        cx.set_menus(menus());
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(920.), px(740.)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("alpha".into()),
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(16.), px(18.))),
                    }),
                    window_min_size: Some(size(px(420.), px(320.))),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| Workspace::new(open, window, cx)),
            )
            .expect("failed to open window");
        window
            .update(cx, |ws, window, cx| {
                window.focus(&ws.focus_handle(cx));
                cx.activate(true);
            })
            .ok();
    });
}
