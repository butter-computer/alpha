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

## Build the Mac app

```sh
scripts/bundle-mac.sh
open target/release/bundle/Alpha.app
```

Builds a release binary and assembles an ad-hoc-signed `Alpha.app` (name, icon, `.md`/`.markdown`/`.txt` document types) so it shows up properly in the Dock and Cmd-Tab. The placeholder icon lives in `assets/icon/`.

## Release DMG

```sh
scripts/make-dmg.sh   # -> target/release/Alpha-<version>-arm64.dmg
```

Runs `bundle-mac.sh`, then packs `Alpha.app` and an `/Applications` shortcut into a compressed DMG (volume `Alpha`, volume icon from `Alpha.icns`, icon-view window with the toolbar hidden; the layout is skipped with a note if Finder scripting is not permitted).

The app is **ad-hoc signed only**: there is no Developer ID, so it is not notarized. On first launch macOS will refuse to open it from a double-click. Drag Alpha to Applications, then **right-click (or Control-click) Alpha.app > Open > Open** once; after that it opens normally. If macOS still blocks it, run `xattr -dr com.apple.quarantine /Applications/Alpha.app`.

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

## Notion-style editing

You type markdown; once a shortcut completes the markers are hidden (not dimmed). The buffer stays plain
markdown, so files save and load as real `.md`.

- Blocks: `# `/`## `/`### ` headings, `- `/`* ` bullets, `1. ` numbered, `[] ` or `- [ ] ` checklist (click the box
  or ⌘Enter to toggle), `> ` quote, ` ``` ` code block, `---` divider.
- Inline: `**bold**` (SemiBold weight), `*italic*`, `` `code` ``, `~~strike~~`, `[text](url)` (⌘-click opens).
- Enter continues a list (empty item exits it); Tab / ⇧Tab indent / outdent list items.
- Backspace right after a hidden marker first reveals it as raw text; moving away re-hides it.
- Type `/` on an empty line for the block menu.

## Layout

- `src/editor/` buffer + undo, layout, custom GPUI element, input handling
- `src/markdown.rs` block scanner + pulldown-cmark inline spans; `src/conceal.rs` conceal layer, offset mapping, backspace reveal
- `src/shortcuts.rs`, `src/slash.rs` typed-shortcut rewrites and the `/` menu
- `src/fonts.rs` bundled default serif (EB Garamond, OFL; see `assets/fonts/OFL.txt`)
- `toolbar.rs`, `palette.rs`, `findbar.rs`, `settings.rs`, `theme.rs`, `export.rs`, `workspace.rs`

## Edge blur (macOS)

The top and bottom window edges get a progressive blur that fades fully into the window background, so scrolling text
is blurred and faded instead of clipped. It is a real Metal pass in a patched copy of gpui (`vendor/gpui`, rationale and
patch in `vendor/README.md`). Tune it via the constants at the top of `src/edge_blur.rs`
(`TOP`, `BOTTOM_EXTRA`, `MAX_RADIUS`, `EDGE_FADE`). The toolbar row is solid background; the blur ramps in the band above it.

## Known gaps

No images or tables. Wrapped list lines don't get a hanging indent.
Open accepts any UTF-8 text file (the native picker is not filtered to .md/.txt).
