# alpha

A light, minimal markdown writing app for macOS, built with Rust and [GPUI](https://crates.io/crates/gpui).
One window, one editor, live markdown rendering in place, and a quiet bottom toolbar.

## Build & run

Requires macOS, a recent Rust toolchain, and Xcode (with the Metal toolchain).

```sh
cargo run --release              # empty document
cargo run --release -- notes.md  # open a file
cargo test                       # unit tests
```

## Shortcuts

| Shortcut | Action |
| --- | --- |
| ⌘K | Command palette |
| ⌘N / ⌘O | New / Open… |
| ⌘S / ⇧⌘S | Save / Save As… |
| ⌘F | Find (Enter / ⇧Enter next / previous, Esc closes) |
| ⌘, | Settings |
| ⌘= / ⌘- | Increase / decrease font size |
| ⌘Q | Quit |
| ⌘Z / ⇧⌘Z | Undo / Redo |
| ⌘A / ⌘C / ⌘X / ⌘V | Select all / copy / cut / paste |
| ⌥←/→, ⌘←/→, ⌘↑/↓ | Move by word / line / document (add ⇧ to select) |
| ⌥⌫ / ⌘⌫ | Delete word / to line start |

Export (Markdown or HTML) is in the toolbar's Export button and the palette.
Settings (theme, font size, font) are saved to `settings.json` in the platform config dir
(`~/Library/Application Support/alpha/` on macOS).

## Layout

- `src/editor/` buffer + undo, layout, custom GPUI element, input handling
- `src/markdown.rs` pulldown-cmark based span analysis
- `src/fonts.rs` bundled default serif (EB Garamond, OFL; see `assets/fonts/OFL.txt`)
- `toolbar.rs`, `palette.rs`, `findbar.rs`, `settings.rs`, `theme.rs`, `export.rs`, `workspace.rs`

## Known gaps

Markdown syntax markers are dimmed, not hidden. No images, tables or nested-list rendering beyond plain styling.
Open accepts any UTF-8 text file (the native picker is not filtered to .md/.txt).
