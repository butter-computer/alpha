# Alpha icon generator

Six "marquetry veneer" icon variants, drawn with p5.js 2.x + p5.brush (WEBGL) in headless Chrome.
Helpers in `helpers.js` / `parquet.js` come from the marquetry-veneer sketch; the 'a' outline is pre-extracted
from the app's EB Garamond (`glyph.js`, regenerate with `extract-glyph.mjs`, needs `opentype.js@1.3.4`).

```sh
node render.mjs out            # all variants -> out/alpha-icon-v*-*.png (1024px, macOS grid, transparent margin)
node render.mjs out v2 v5      # a subset
node contact.mjs out           # contact sheet with 128px / 32px renders
```

Needs the `agent-browser` CLI and network access (pinned CDN scripts). Seeds are pinned (`?seed=` overrides).
Open `index.html?v=v1&preview=1` through any static server to preview one variant.
