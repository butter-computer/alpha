# vendor/gpui

A copy of the crates.io `gpui` 0.2.2 crate (same release the app was built on) with one patch: a
progressive edge blur in the macOS Metal renderer. Cargo is pointed at it with `[patch.crates-io]`
in the root `Cargo.toml`. Everything else (`gpui_*` helper crates) still comes from crates.io.

Changes against upstream:
- `examples/` removed and the matching `[[example]]` entries dropped from `Cargo.toml` (size only).
- `vendor/gpui-edge-blur.patch` is the full diff of the real change (renderer, shaders, scene, window).

## Why patch gpui

Native `NSVisualEffectView` bands over the window cannot blur GPUI's Metal layer, and a
post-process has to run inside the renderer. A frame has to be split in two so chrome like the
toolbar stays sharp on top of the blurred content.

## What the patch does

- `set_edge_blur(Some(EdgeBlurConfig { top, bottom, bottom_solid, max_radius, tint, tint_opacity }))`
  (points; `None` disables it, and then rendering is exactly upstream's path).
- `Window::paint_edge_blur_split()` paints a transparent full-window marker quad. Primitives
  with a draw order up to the marker are rendered to an offscreen texture; primitives after it are
  drawn sharp on top of the blurred result (the toolbar, popups, palette).
- Blur passes (`shaders.metal`, `metal_renderer.rs::encode_edge_blur`):
  1. horizontal Gaussian into a temp texture, only for the band rows (scissored);
  2. vertical Gaussian over those rows, blended toward the window background tint, composited with
     the untouched middle of the scene into the drawable.
- The radius and the tint opacity both ease (smoothstep) from 0 at the inner edge of each band to
  `max_radius` / `tint_opacity` at the window edge. `bottom_solid` rows at the very bottom (the
  toolbar row) are always fully blurred and tinted so nothing shows through behind the toolbar.
- Scale factor (Retina) is applied from the layer's `contentsScale`; light/dark is just the tint
  colour the app passes in.

App side: `src/edge_blur.rs` (constants `TOP`, `BOTTOM_EXTRA`, `MAX_RADIUS`, `EDGE_FADE`).

## Updating gpui

Re-copy the new crate, then `patch -p1` from the diff (or re-apply by hand; it touches
`src/platform/mac/{metal_renderer.rs,shaders.metal}`, `src/{scene,window}.rs`, `src/platform.rs`).
