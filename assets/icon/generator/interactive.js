// Icon playground: the rosewood-oval icon as a fully parametric marquetry generator.
// Art (p5.brush, WEBGL) re-renders on art-setting changes; tile settings only re-compose the cached art.
// Fonts: the app's EB Garamond (local) plus OFL serifs from fontsource via jsDelivr.
const FONTS = [
  ["Playfair Display", "playfair-display", [400, 500, 600, 700, 800, 900]],
  ["EB Garamond", "eb-garamond", [400, 500, 600, 700, 800]],
  ["EB Garamond (app file)", "local", [400, 500]],
  ["Cormorant Garamond", "cormorant-garamond", [400, 500, 600, 700]],
  ["Lora", "lora", [400, 500, 600, 700]],
  ["Libre Baskerville", "libre-baskerville", [400, 700]],
  ["Baskervville", "baskervville", [400, 500, 600, 700]],
  ["Libre Caslon Text", "libre-caslon-text", [400, 700]],
  ["Libre Bodoni", "libre-bodoni", [400, 500, 600, 700]],
  ["Bodoni Moda", "bodoni-moda", [400, 500, 600, 700, 800, 900]],
  ["DM Serif Display", "dm-serif-display", [400]],
  ["Abril Fatface", "abril-fatface", [400]],
  ["Fraunces", "fraunces", [400, 500, 600, 700, 800]],
  ["Newsreader", "newsreader", [400, 500, 600, 700]],
  ["Crimson Pro", "crimson-pro", [400, 500, 600, 700]],
  ["Source Serif 4", "source-serif-4", [400, 500, 600, 700]],
  ["Spectral", "spectral", [400, 500, 600, 700]],
  ["Vollkorn", "vollkorn", [400, 500, 600, 700]],
  ["Gelasio", "gelasio", [400, 500, 600, 700]],
  ["Cardo", "cardo", [400, 700]],
  ["Old Standard TT", "old-standard-tt", [400, 700]],
  ["Noto Serif Display", "noto-serif-display", [400, 500, 600, 700]],
  ["Instrument Serif", "instrument-serif", [400]],
  ["Young Serif", "young-serif", [400]],
  ["Gloock", "gloock", [400]],
  ["Prata", "prata", [400]],
  ["Yeseva One", "yeseva-one", [400]],
  ["Rozha One", "rozha-one", [400]],
  ["Marcellus", "marcellus", [400]],
  ["Cinzel", "cinzel", [400, 500, 600, 700]],
];
const LOCAL = { 400: "../../fonts/EBGaramond-Regular.ttf", 500: "../../fonts/EBGaramond-Medium.ttf" };

// ---- palettes -------------------------------------------------------------------------------
const PAL_KEYS = ["letterBase", "letterEdge", "letterGDark", "letterGLight", "groundBase", "groundEdge", "groundGDark", "groundGLight",
  "fieldBase", "counterBase", "ringLight", "ringDark", "ringSeam", "seam", "bg", "shadowColor"];
const PRESETS = {
  "Original rosewood + maple": { letterBase: "#dfc694", letterEdge: "#8a6544", letterGDark: "#967747", letterGLight: "#fff0c2", groundBase: "#4f3228", groundEdge: "#1f1613", groundGDark: "#1e1512", groundGLight: "#b07a55", fieldBase: "#3d281f", counterBase: "#4b3026", ringLight: "#b18a58", ringDark: "#694235", ringSeam: "#2b201a", seam: "#241915", bg: "#34261f", shadowColor: "#000000" },
  "Ebony + holly": { letterBase: "#f2eee2", letterEdge: "#a89f8c", letterGDark: "#cfc6b0", letterGLight: "#ffffff", groundBase: "#2a2523", groundEdge: "#0e0c0b", groundGDark: "#0a0908", groundGLight: "#5a524c", fieldBase: "#1b1816", counterBase: "#262120", ringLight: "#e8e2d0", ringDark: "#3a322d", ringSeam: "#0b0a09", seam: "#0b0a09", bg: "#1a1715", shadowColor: "#000000" },
  "Walnut + ash": { letterBase: "#d9cdb4", letterEdge: "#8f8268", letterGDark: "#a89a7c", letterGLight: "#f6eedb", groundBase: "#5a4332", groundEdge: "#2b1f16", groundGDark: "#2a1d14", groundGLight: "#9a7b5c", fieldBase: "#46342a", counterBase: "#52392b", ringLight: "#c4b393", ringDark: "#4a3526", ringSeam: "#2b1f16", seam: "#2b1f16", bg: "#2f2118", shadowColor: "#000000" },
  "Cherry + birch": { letterBase: "#ecd9b0", letterEdge: "#a77d4e", letterGDark: "#c9a46d", letterGLight: "#fff3d0", groundBase: "#7a3b2b", groundEdge: "#3a1812", groundGDark: "#3a1710", groundGLight: "#c4705a", fieldBase: "#5f2b20", counterBase: "#6b3224", ringLight: "#d8b983", ringDark: "#5a261c", ringSeam: "#2a100b", seam: "#2a100b", bg: "#3a1a13", shadowColor: "#000000" },
  "Olive + bone": { letterBase: "#ece4d0", letterEdge: "#a49c84", letterGDark: "#bdb59c", letterGLight: "#fffaf0", groundBase: "#4d5236", groundEdge: "#20240f", groundGDark: "#1c2010", groundGLight: "#8d9566", fieldBase: "#3a3f27", counterBase: "#454a30", ringLight: "#cfc7ad", ringDark: "#333820", ringSeam: "#1a1d0d", seam: "#1a1d0d", bg: "#262a18", shadowColor: "#000000" },
  "Monochrome dark": { letterBase: "#d8d8dc", letterEdge: "#7a7a82", letterGDark: "#9a9aa2", letterGLight: "#ffffff", groundBase: "#2e2e30", groundEdge: "#121213", groundGDark: "#111112", groundGLight: "#6a6a70", fieldBase: "#212123", counterBase: "#2a2a2c", ringLight: "#9a9aa0", ringDark: "#3a3a3e", ringSeam: "#0d0d0e", seam: "#0d0d0e", bg: "#1a1a1c", shadowColor: "#000000" },
  "Light / inverted": { letterBase: "#5a362a", letterEdge: "#241813", letterGDark: "#2a1914", letterGLight: "#a8745a", groundBase: "#d9c096", groundEdge: "#8f7350", groundGDark: "#8a6a43", groundGLight: "#f6e8c4", fieldBase: "#c9ad7f", counterBase: "#d0b688", ringLight: "#8a5a46", ringDark: "#e8d4a6", ringSeam: "#3a2518", seam: "#3a2518", bg: "#cdb487", shadowColor: "#3a2515" },
};

// ---- settings (defaults = the shipped rosewood-oval Playfair icon) --------------------------------
const DEFAULTS = {
  ...PRESETS["Original rosewood + maple"],
  font: "playfair-display", weight: 700, char: "a", center: "blend", size: 560, dx: 0, dy: 0, rot: 0,
  style: "single", pieces: 5, pieceTilt: -22, outlineW: 3.5,
  lAngle: 83, lSpacing: 7, lAmp: 3.2, lOpacity: 1, lWeight: 0.27,
  gAngle: 8, gSpacing: 9, gAmp: 5, gOpacity: 1, gWeight: 0.27, seed: 41,
  depth: 9, steps: 5, edgeDark: 0, seamW: 1, halo: 8, gInset: 0.045,
  ring: true, ringShape: "oval", ringR: 500, ringT: 18, ringCount: 56, ringPattern: "pairs", ringRot: 0, field: true,
  ground: "flat", wedges: 32,
  tileStyle: "squircle", squircleN: 5, cornerR: 180, margin: 100, shadowBlur: 24, shadowY: 10, shadowOpacity: 0.30, rim: false,
};
const S = JSON.parse(JSON.stringify(DEFAULTS));
const TILE = new Set(["tileStyle", "squircleN", "cornerR", "margin", "shadowBlur", "shadowY", "shadowOpacity", "shadowColor", "rim"]);

const R_ = (k, l, min, max, step) => ({ k, t: "range", l, min, max, step });
const C_ = (k, l) => ({ k, t: "color", l });
const SEL = (k, l, o) => ({ k, t: "select", l, o });
const SCHEMA = [
  ["Palette", true, [{ t: "presets" },
    C_("letterBase", "Letter veneer"), C_("letterEdge", "Letter edge shade"), C_("letterGDark", "Letter grain dark"), C_("letterGLight", "Letter grain light"),
    C_("groundBase", "Tile veneer"), C_("groundEdge", "Tile edge shade"), C_("groundGDark", "Tile grain dark"), C_("groundGLight", "Tile grain light"),
    C_("fieldBase", "Inner field"), C_("counterBase", "Letter counter"),
    C_("ringLight", "Ring light band"), C_("ringDark", "Ring dark band"), C_("ringSeam", "Ring seams"),
    C_("seam", "Seams / halo"), C_("bg", "Background tint"), C_("shadowColor", "Drop shadow")]],
  ["Letter", true, [{ k: "char", t: "text", l: "Glyph" }, { k: "font", t: "font", l: "Font" }, { k: "weight", t: "weight", l: "Weight" },
    SEL("center", "Centering", [["blend", "Optical (blend)"], ["centroid", "Visual mass"], ["bbox", "Bounding box"]]),
    R_("size", "Size", 200, 900, 5), R_("dx", "Offset X", -200, 200, 1), R_("dy", "Offset Y", -200, 200, 1), R_("rot", "Rotation", -180, 180, 1),
    SEL("style", "Veneer style", [["single", "Single piece"], ["pieced", "Pieced grain"], ["outlined", "Outlined"]]),
    R_("pieces", "Pieces (pieced)", 2, 9, 1), R_("pieceTilt", "Cut angle (pieced)", -90, 90, 1), R_("outlineW", "Line weight (outlined)", 0.5, 8, 0.1)]],
  ["Grain · letter", false, [R_("lAngle", "Angle", -90, 90, 1), R_("lSpacing", "Spacing", 3, 24, 0.5), R_("lAmp", "Wave amplitude", 0, 14, 0.2), R_("lOpacity", "Opacity", 0, 2, 0.05), R_("lWeight", "Line weight", 0.1, 0.8, 0.01)]],
  ["Grain · ground", false, [R_("gAngle", "Angle", -90, 90, 1), R_("gSpacing", "Spacing", 4, 30, 0.5), R_("gAmp", "Wave amplitude", 0, 14, 0.2), R_("gOpacity", "Opacity", 0, 2, 0.05), R_("gWeight", "Line weight", 0.1, 0.8, 0.01), R_("seed", "Seed", 1, 99, 1)]],
  ["Shading", false, [R_("depth", "Inset depth (letter)", 0, 30, 1), R_("steps", "Shading steps", 1, 10, 1), R_("edgeDark", "Edge darkness", 0, 1, 0.02), R_("seamW", "Seam weight", 0, 3, 0.05), R_("halo", "Letter halo gap", 0, 24, 1), R_("gInset", "Tile edge inset", 0, 0.12, 0.005)]],
  ["Ring / border", true, [{ k: "ring", t: "check", l: "Ring on" },
    SEL("ringShape", "Shape", [["oval", "Oval"], ["circle", "Circle"], ["square", "Square inlay"], ["none", "None"]]),
    R_("ringR", "Radius", 250, 640, 5), R_("ringT", "Thickness", 4, 70, 1), R_("ringCount", "Band count", 8, 120, 1),
    SEL("ringPattern", "Band pattern", [["pairs", "Alternating pairs"], ["alt", "Alternating singles"], ["sunburst", "Sunburst (tapered)"]]),
    R_("ringRot", "Ring rotation", 0, 360, 1), { k: "field", t: "check", l: "Inner field" }]],
  ["Ground", false, [SEL("ground", "Style", [["flat", "Flat veneer"], ["parquet", "Parquet"], ["sunburst", "Sunburst rosette"], ["plain", "Plain"]]), R_("wedges", "Sunburst wedges", 8, 64, 1)]],
  ["Tile", false, [SEL("tileStyle", "Corners", [["squircle", "Squircle"], ["rect", "Rounded rect"]]), R_("squircleN", "Squircle exponent", 2, 12, 0.1), R_("cornerR", "Rect corner radius", 0, 412, 2),
    R_("margin", "Margin", 0, 200, 2), R_("shadowBlur", "Shadow size", 0, 80, 1), R_("shadowY", "Shadow offset", 0, 40, 1), R_("shadowOpacity", "Shadow opacity", 0, 1, 0.02), { k: "rim", t: "check", l: "Highlight rim" }]],
];

// ---- colour + geometry helpers --------------------------------------------------------------------
const hx = n => Math.max(0, Math.min(255, Math.round(n))).toString(16).padStart(2, "0");
const toRGB = h => [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16));
const mixHex = (a, b, t) => { const A = toRGB(a), B = toRGB(b); return "#" + A.map((v, i) => hx(v + (B[i] - v) * t)).join(""); };
function hsl(h, s, l) { const f = n => { const k = (n + h / 30) % 12, a = s * Math.min(l, 1 - l); return l - a * Math.max(-1, Math.min(k - 3, 9 - k, 1)); }; return "#" + [f(0), f(8), f(4)].map(v => hx(v * 255)).join(""); }
const lum = h => { const [r, g, b] = toRGB(h); return (0.299 * r + 0.587 * g + 0.114 * b) / 255; };
const shades = h => [h, mixHex(h, "#ffffff", 0.06), mixHex(h, "#000000", 0.07), mixHex(h, "#ffffff", 0.12)];
const alt = h => mixHex(h, lum(h) > 0.4 ? "#000000" : "#ffffff", 0.05);
const rg = (hex, a) => rgba(hex, Math.max(0, Math.min(255, Math.round(a))));
const ed = hex => mixHex(hex, "#000000", S.edgeDark);
const rnd = (a, b) => a + window.__uiRandom() * (b - a);   // UI only (unseeded); art uses the seeded PRNG

function randomPalette() {
  const h = rnd(0, 360), h2 = (h + rnd(-40, 40) + 360) % 360, inv = window.__uiRandom() < 0.25;
  const gs = rnd(0.2, 0.5), ls = rnd(0.4, 0.55);
  let ground = hsl(h, gs, rnd(0.2, 0.42)), letter = hsl(h2, ls * 0.8, rnd(0.74, 0.87));
  if (inv) [ground, letter] = [hsl(h2, ls * 0.8, rnd(0.74, 0.84)), hsl(h, gs, rnd(0.18, 0.3))];
  const dk = (c, t) => mixHex(c, "#000000", t), lt = (c, t) => mixHex(c, "#ffffff", t);
  const dark = lum(ground) < 0.5;
  return {
    letterBase: letter, letterEdge: dk(letter, 0.5), letterGDark: dk(letter, 0.3), letterGLight: lt(letter, 0.6),
    groundBase: ground, groundEdge: dk(ground, 0.6), groundGDark: dk(ground, 0.7), groundGLight: lt(ground, dark ? 0.35 : 0.55),
    fieldBase: dk(ground, 0.22), counterBase: dk(ground, 0.06),
    ringLight: dk(letter, 0.22), ringDark: dark ? lt(ground, 0.05) : dk(ground, 0.45), ringSeam: dk(ground, 0.7), seam: dk(ground, 0.65), bg: dk(ground, 0.35),
    shadowColor: dark ? "#000000" : dk(ground, 0.75),
  };
}

function polyPts(cx, cy, rx, ry, n, count, rotDeg = 0) {   // superellipse (n=2 → ellipse), CCW in screen coords
  const pts = [];
  for (let i = 0; i < count; i++) pts.push(spPoint(cx, cy, rx, ry, n, i * 360 / count + rotDeg));
  return pts;
}
function spPoint(cx, cy, rx, ry, n, deg) {
  const t = deg * Math.PI / 180, c = Math.cos(t), s = Math.sin(t);
  return [cx + rx * Math.sign(c) * Math.pow(Math.abs(c), 2 / n), cy + ry * Math.sign(s) * Math.pow(Math.abs(s), 2 / n)];
}

// ---- font + glyph ---------------------------------------------------------------------------------
let glyph = null, busy = false, again = false;
const fontCache = {};
function flatten(cmds, steps = 14) {
  const out = []; let cur = null, px = 0, py = 0;
  for (const c of cmds) {
    if (c.type === "M") { cur = [[c.x, c.y]]; out.push(cur); px = c.x; py = c.y; }
    else if (c.type === "L") { cur.push([c.x, c.y]); px = c.x; py = c.y; }
    else if (c.type === "Q") { for (let i = 1; i <= steps; i++) { const t = i / steps, u = 1 - t; cur.push([u*u*px + 2*u*t*c.x1 + t*t*c.x, u*u*py + 2*u*t*c.y1 + t*t*c.y]); } px = c.x; py = c.y; }
    else if (c.type === "C") { for (let i = 1; i <= steps; i++) { const t = i / steps, u = 1 - t; cur.push([u*u*u*px + 3*u*u*t*c.x1 + 3*u*t*t*c.x2 + t*t*t*c.x, u*u*u*py + 3*u*u*t*c.y1 + 3*u*t*t*c.y2 + t*t*t*c.y]); } px = c.x; py = c.y; }
  }
  return out.filter(c => c.length > 2);
}
function fontUrl() {
  return S.font === "local" ? LOCAL[S.weight] : `https://cdn.jsdelivr.net/npm/@fontsource/${S.font}/files/${S.font}-latin-${S.weight}-normal.woff`;
}
function fetchFont(url) { return fontCache[url] ??= new Promise((res, rej) => opentype.load(url, (e, f) => e ? rej(e) : res(f))); }

function centroid(poly) {
  let a = 0, cx = 0, cy = 0;
  for (let i = 0; i < poly.length; i++) { const [x0, y0] = poly[i], [x1, y1] = poly[(i + 1) % poly.length], k = x0 * y1 - x1 * y0; a += k; cx += (x0 + x1) * k; cy += (y0 + y1) * k; }
  return [cx / (3 * a), cy / (3 * a), a / 2];
}
function place(contours) {
  const all = contours.flat();
  const xs = all.map(p => p[0]), ys = all.map(p => p[1]);
  const x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
  const s = S.size / Math.max(x1 - x0, y1 - y0);
  let A = 0, CX = 0, CY = 0;
  for (const c of contours) { const [cx, cy, a] = centroid(c); A += a; CX += cx * a; CY += cy * a; }
  CX /= A; CY /= A;
  const bx = (x0 + x1) / 2, by = (y0 + y1) / 2, t = { bbox: 0, blend: 0.5, centroid: 1 }[S.center];
  const mx = bx + (CX - bx) * t, my = by + (CY - by) * t;
  const r = S.rot * Math.PI / 180, cr = Math.cos(r), sr = Math.sin(r), ox = 600 + S.dx, oy = 600 + S.dy;
  const placed = contours.map(c => c.map(p => {
    const x = (p[0] - mx) * s, y = (p[1] - my) * s;
    return [ox + x * cr - y * sr, oy + x * sr + y * cr];
  }));
  placed.sort((a, b) => Math.abs(polyArea(b)) - Math.abs(polyArea(a)));
  return { outer: placed[0], holes: placed.slice(1) };
}

// ---- drawing --------------------------------------------------------------------------------------
const RECT = [[0, 0], [1200, 0], [1200, 1200], [0, 1200]];
const gGrain = (poly, angle, ph = 0) => {
  grainInShape(poly, angle, S.gSpacing, rg(S.groundGDark, 120 * S.gOpacity), S.gWeight, S.gAmp, 0.9 + ph);
  grainInShape(poly, angle, S.gSpacing * 2.55, rg(S.groundGLight, 40 * S.gOpacity), S.gWeight * 0.8, S.gAmp * 0.8, 2.4 + ph);
};

function drawGround() {
  if (S.ground === "parquet") {
    window.PARQUET = { satin: shades(S.ringLight), rose: shades(mixHex(S.groundBase, "#ffffff", 0.12)) };
    drawParquet();
  } else if (S.ground === "sunburst") {
    const n = S.wedges, light = shades(S.ringLight), dark = shades(mixHex(S.groundBase, "#ffffff", 0.1));
    for (let i = 0; i < n; i++) {
      const a0 = i * 360 / n - 90 - 180 / n, a1 = a0 + 360 / n, pts = [[600, 600]];
      for (let k = 0; k <= 4; k++) pts.push(spPoint(600, 600, 1000, 1000, 2, lerp(a0, a1, k / 4)));
      const isLight = i % 2 === 0, base = (isLight ? light : dark)[Math.floor(i / 2) % 4];
      drawPoly(pts, mixHex(base, "#2d201a", 0.34), S.seam, 0.24 * S.seamW);
      const inner = scalePoly(pts, 0.985);
      drawPoly(inner, base, null);
      grainInShape(inner, (a0 + a1) / 2, S.gSpacing, rg(isLight ? S.letterGDark : S.groundGDark, 80 * S.gOpacity), S.gWeight * 0.75, S.gAmp * 0.45, i * 1.3);
      grainInShape(inner, (a0 + a1) / 2, S.gSpacing * 2.7, rg(isLight ? S.letterGLight : S.groundGLight, 44 * S.gOpacity), S.gWeight * 0.7, S.gAmp * 0.4, i * 0.7);
    }
  } else {
    shadedPiece(RECT, S.groundBase, ed(S.groundEdge), S.ground === "plain" ? 0.012 : S.gInset, S.ground === "plain" ? 2 : S.steps, S.seam, 0.4 * S.seamW);
    if (S.ground === "flat") gGrain(RECT, S.gAngle);
  }
}

function ringGeom() {
  const R = S.ringR, sq = S.ringShape === "square";
  const ry = S.ringShape === "oval" ? R * 1.05 : R, n = sq ? 6 : 2;
  return { rx: R, ry, irx: R - S.ringT, iry: ry - S.ringT, n };
}

function drawRingAndField() {
  const g = ringGeom();
  if (S.ring && S.ringShape !== "none") {
    const N = S.ringCount, outerPts = [], bandPoint = (rx, ry, a) => spPoint(600, 600, rx, ry, g.n, a + S.ringRot);
    for (let i = 0; i < N; i++) {
      const a0 = i * 360 / N, a1 = (i + 1) * 360 / N;
      const sun = S.ringPattern === "sunburst";
      const ia0 = sun ? lerp(a0, a1, 0.28) : a0, ia1 = sun ? lerp(a0, a1, 0.72) : a1;
      const pts = [];
      for (let k = 0; k <= 3; k++) pts.push(bandPoint(g.rx, g.ry, lerp(a0, a1, k / 3)));
      for (let k = 3; k >= 0; k--) pts.push(bandPoint(g.irx, g.iry, lerp(ia0, ia1, k / 3)));
      const light = S.ringPattern === "pairs" ? (Math.floor(i / 2) % 2) === 0 : i % 2 === 0;
      const base = light ? (i % 2 ? alt(S.ringLight) : S.ringLight) : (i % 2 ? alt(S.ringDark) : S.ringDark);
      drawPoly(pts, mixHex(base, "#281d18", 0.3), S.ringSeam, 0.24 * S.seamW);
      drawPoly(scalePoly(pts, 0.87), base, null);
      const mid = [];
      for (let k = 0; k <= 4; k++) mid.push(bandPoint((g.rx + g.irx) / 2, (g.ry + g.iry) / 2, lerp(a0 + 0.25, a1 - 0.25, k / 4)));
      brush.set("rotring", light ? rg(mixHex(S.ringLight, "#000000", 0.6), 70) : rg(mixHex(S.ringDark, "#ffffff", 0.45), 40), 0.18);
      brush.spline(mid, 0.25);
    }
    brush.noFill(); brush.noWash();
    const loop = (rx, ry) => { const p = polyPts(600, 600, rx, ry, g.n, 144, S.ringRot); p.push(p[0]); return p; };
    brush.set("rotring", rg(S.seam, 190), 0.42 * S.seamW);
    brush.spline(loop(g.rx, g.ry), 0.2);
    brush.set("rotring", rg(mixHex(S.ringLight, "#ffffff", 0.2), 85), 0.26);
    brush.spline(loop(g.irx, g.iry), 0.2);
  }
  if (S.field) {
    const field = polyPts(600, 600, g.irx - 4, g.iry - 4, g.n, 120, S.ringRot);
    shadedPiece(field, S.fieldBase, ed(mixHex(S.fieldBase, "#000000", 0.55)), 0.05, S.steps, ed(mixHex(S.fieldBase, "#000000", 0.55)), 0.4 * S.seamW);
    gGrain(field, S.gAngle - 12, 2.4);
  }
}

function letterWood(base, edge) { return { base, edge, dark: rg(S.letterGDark, 55 * S.lOpacity), light: rg(S.letterGLight, 65 * S.lOpacity) }; }

function letterPiece(outer, holes, wood, angle, phase, depth) {
  shadedInset(outer, wood.base, ed(wood.edge), depth, S.steps, S.seam, 0.46 * S.seamW);
  grainInShape(outer, angle, S.lSpacing, wood.dark, S.lWeight, S.lAmp, phase, holes);
  grainInShape(outer, angle, S.lSpacing * 2.7, wood.light, S.lWeight * 0.85, S.lAmp * 0.95, phase * 1.7 + 0.8, holes);
}

function drawLetter() {
  const g = place(glyph);
  const wood = letterWood(S.letterBase, S.letterEdge);
  if (S.style === "outlined") {
    brush.noFill(); brush.noWash();
    brush.set("rotring", rg(S.letterBase, 255), S.outlineW);
    outlinePoly(g.outer);
    for (const h of g.holes) outlinePoly(h);
    brush.set("rotring", rg(S.letterEdge, 200), S.outlineW * 0.45);
    outlinePoly(insetPoly(g.outer, 11));
    return;
  }
  if (S.halo > 0) drawPoly(insetPoly(g.outer, -S.halo), S.seam, null);
  if (S.style === "single") {
    letterPiece(g.outer, g.holes, wood, S.lAngle, 0.4, S.depth);
  } else {
    const xs = g.outer.map(p => p[0]), ys = g.outer.map(p => p[1]);
    const x0 = Math.min(...xs), x1 = Math.max(...xs), span = x1 - x0, cx = (x0 + x1) / 2, cy = (Math.min(...ys) + Math.max(...ys)) / 2;
    const tilt = S.pieceTilt * Math.PI / 180, dir = [Math.cos(tilt), Math.sin(tilt)], nrm = [-dir[1], dir[0]];
    const N = S.pieces, cuts = [-1e4];
    for (let i = 1; i < N; i++) cuts.push((i / N - 0.5 + (((i * 37) % 11) - 5) / 120) * span);
    cuts.push(1e4);
    const tone = [0.04, -0.07, 0.1, -0.12, 0.02, 0.07, -0.04, 0.12, -0.09], angOff = [0, -111, -35, -151, -71, -20, -95, -55, -130];
    for (let i = 0; i < N; i++) {
      const q = [], at = (o, s) => [cx + nrm[0] * o + dir[0] * s, cy + nrm[1] * o + dir[1] * s];
      q.push(at(cuts[i], -2000), at(cuts[i], 2000), at(cuts[i + 1], 2000), at(cuts[i + 1], -2000));
      const base = mixHex(S.letterBase, tone[i % 9] > 0 ? "#ffffff" : "#000000", Math.abs(tone[i % 9]));
      polygonClipping.intersection([g.outer], [q]).forEach((poly, j) => {
        const ring = poly[0].slice(0, -1);
        if (Math.abs(polyArea(ring)) < 200) return;
        letterPiece(ring, [], letterWood(base, S.letterEdge), S.lAngle + angOff[i % 9], i * 1.9 + j, S.depth * 0.66);
      });
    }
    brush.noFill(); brush.noWash();
    brush.set("rotring", rg(S.seam, 200), 0.5 * S.seamW);
    outlinePoly(g.outer);
  }
  const cEdge = mixHex(S.counterBase, "#000000", 0.45);
  for (const h of g.holes) {
    shadedPiece(h, S.counterBase, ed(cEdge), 0.07, Math.max(2, S.steps - 1), S.seam, 0.42 * S.seamW);
    grainInShape(h, 10, 6.5, rg(S.groundGDark, 145 * S.gOpacity), 0.26, 2.4, 1.3);
    grainInShape(h, 10, 18, rg(S.groundGLight, 44 * S.gOpacity), 0.2, 2.2, 2.3);
  }
}

function drawIcon() {
  brush.noStroke(); brush.wash(S.bg, 255); brush.rect(0, 0, 1200, 1200); brush.noWash();
  drawGround();
  drawRingAndField();
  drawLetter();
}

// ---- tile composition (cheap; runs on tile-setting changes without re-rendering the art) -----------
const art = document.createElement("canvas"); art.width = art.height = 1200;
let artReady = false;

function tilePath(ctx, half, inset = 0) {
  const h = half - inset;
  ctx.beginPath();
  if (S.tileStyle === "rect") { ctx.roundRect(512 - h, 512 - h, 2 * h, 2 * h, Math.min(S.cornerR, h)); return; }
  for (let i = 0; i <= 360; i++) {
    const [x, y] = spPoint(512, 512, h, h, S.squircleN, i);
    i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
  }
  ctx.closePath();
}

function compose() {
  if (!artReady) return;
  const out = document.getElementById("out"), ctx = out.getContext("2d"), half = 512 - S.margin;
  ctx.clearRect(0, 0, 1024, 1024);
  ctx.imageSmoothingQuality = "high";
  const [r, g, b] = toRGB(S.shadowColor);
  ctx.save(); ctx.shadowColor = `rgba(${r},${g},${b},${S.shadowOpacity})`; ctx.shadowBlur = S.shadowBlur; ctx.shadowOffsetY = S.shadowY;
  tilePath(ctx, half); ctx.fillStyle = "#000"; ctx.fill(); ctx.restore();
  ctx.save(); tilePath(ctx, half); ctx.clip(); ctx.drawImage(art, 512 - half, 512 - half, 2 * half, 2 * half); ctx.restore();
  tilePath(ctx, half, 1); ctx.lineWidth = 2; ctx.strokeStyle = "rgba(0,0,0,0.22)"; ctx.stroke();
  if (S.rim) {
    const grad = ctx.createLinearGradient(0, 512 - half, 0, 512 + half);
    grad.addColorStop(0, "rgba(255,255,255,0.55)"); grad.addColorStop(0.4, "rgba(255,255,255,0.06)"); grad.addColorStop(1, "rgba(255,255,255,0)");
    tilePath(ctx, half, 3); ctx.lineWidth = 3; ctx.strokeStyle = grad; ctx.stroke();
  }
  const url = out.toDataURL("image/png");
  for (const id of ["p128", "p64", "p32"]) document.getElementById(id).src = url;
  window.__png = url;
}

// ---- p5 lifecycle ---------------------------------------------------------------------------------
async function setup() {
  createCanvas(1200, 1200, WEBGL);
  brush.scaleBrushes(6);
  angleMode(DEGREES);
  noLoop();
  const q = new URLSearchParams(location.search).get("s");
  if (q) try { Object.assign(S, JSON.parse(atob(q))); } catch (e) { console.warn("bad s param", e); }
  buildUI();
  render();
}
function draw() {
  if (!glyph) return;
  randomSeed(S.seed); noiseSeed(S.seed);
  background(S.bg);
  translate(-width / 2, -height / 2);
  brush.noField();
  drawIcon();
}

async function render() {
  if (busy) { again = true; return; }
  busy = true;
  const status = document.getElementById("status");
  try {
    status.textContent = "loading font…";
    const font = await fetchFont(fontUrl());
    glyph = flatten(font.getPath(S.char || "a", 0, 0, 1000).commands);
    if (!glyph.length) throw new Error("that font has no outline for “" + S.char + "”");
    status.textContent = "rendering…";
    await new Promise(r => setTimeout(r, 30));
    const t0 = performance.now();
    await redraw();
    art.getContext("2d").clearRect(0, 0, 1200, 1200);
    art.getContext("2d").drawImage(document.querySelector("canvas:not(#out)"), 0, 0, 1200, 1200);
    artReady = true;
    compose();
    status.textContent = `rendered in ${((performance.now() - t0) / 1000).toFixed(1)}s`;
    if (new URLSearchParams(location.search).get("auto")) {
      const b = await (await fetch(window.__png)).blob();
      await fetch("/save?name=play", { method: "POST", body: b });
      window.__done = true;
    }
  } catch (e) {
    status.textContent = "⚠ " + (e.message || e);
  }
  busy = false;
  if (again) { again = false; render(); }
}
let timer;
const later = () => { clearTimeout(timer); timer = setTimeout(render, 250); };

// ---- UI -------------------------------------------------------------------------------------------
const ctl = {};
function h(tag, attrs = {}, ...kids) {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) k === "class" ? e.className = v : k.startsWith("on") ? e[k] = v : e.setAttribute(k, v);
  for (const c of kids) e.append(c);
  return e;
}
function changed(k) { TILE.has(k) ? compose() : later(); }

function buildRow(it) {
  const k = it.k, row = h("div", { class: "row" }, h("label", {}, it.l));
  let input, show;
  if (it.t === "color") {
    input = h("input", { type: "color" }); show = h("code");
    input.oninput = () => { S[k] = input.value; show.textContent = input.value; changed(k); };
    ctl[k] = v => { input.value = v; show.textContent = v; };
    row.append(input, show);
  } else if (it.t === "range") {
    input = h("input", { type: "range", min: it.min, max: it.max, step: it.step }); show = h("code");
    input.oninput = () => { S[k] = +input.value; show.textContent = input.value; changed(k); };
    ctl[k] = v => { input.value = v; show.textContent = v; };
    row.append(input, show);
  } else if (it.t === "select" || it.t === "font" || it.t === "weight") {
    input = h("select");
    if (it.t === "select") it.o.forEach(([v, l]) => input.add(new Option(l, v)));
    if (it.t === "font") FONTS.forEach(f => input.add(new Option(f[0], f[1])));
    input.onchange = () => {
      S[k] = it.t === "weight" ? +input.value : input.value;
      if (it.t === "font") { fillWeights(); }
      changed(k);
    };
    ctl[k] = v => { input.value = v; };
    row.append(input);
  } else if (it.t === "check") {
    input = h("input", { type: "checkbox" });
    input.onchange = () => { S[k] = input.checked; changed(k); };
    ctl[k] = v => { input.checked = !!v; };
    row.append(input);
  } else if (it.t === "text") {
    input = h("input", { type: "text", maxlength: 1 });
    input.oninput = () => { S[k] = input.value.slice(0, 1) || "a"; later(); };
    ctl[k] = v => { input.value = v; };
    row.append(input);
  }
  return row;
}

function fillWeights() {
  const sel = document.querySelector("[data-w]") || null;
  const f = FONTS.find(x => x[1] === S.font) || FONTS[0], ws = f[2], wsel = ctl.__weightEl;
  wsel.innerHTML = ""; ws.forEach(w => wsel.add(new Option(w, w)));
  if (!ws.includes(S.weight)) S.weight = ws.reduce((a, b) => Math.abs(b - S.weight) < Math.abs(a - S.weight) ? b : a);
  wsel.value = S.weight;
}

function syncUI() {
  for (const k of Object.keys(ctl)) if (k in S) ctl[k](S[k]);
  fillWeights();
  document.getElementById("json").value = JSON.stringify(S);
}

function applyPalette(p) { Object.assign(S, p); syncUI(); render(); }

function buildUI() {
  const root = document.getElementById("groups");
  for (const [title, open, items] of SCHEMA) {
    const body = h("div", { class: "body" }), det = h("details", open ? { open: "" } : {}, h("summary", {}, title), body);
    for (const it of items) {
      if (it.t === "presets") {
        const sel = h("select");
        sel.add(new Option("Palette presets…", ""));
        Object.keys(PRESETS).forEach(n => sel.add(new Option(n, n)));
        sel.onchange = () => { if (sel.value) applyPalette(PRESETS[sel.value]); sel.value = ""; };
        body.append(h("div", { class: "row wide" }, sel, h("button", { class: "ghost", onclick: () => applyPalette(randomPalette()) }, "🎲 Randomize")));
      } else {
        const row = buildRow(it);
        if (it.t === "weight") ctl.__weightEl = row.querySelector("select");
        body.append(row);
      }
    }
    root.append(det);
  }
  // settings / export group
  const ta = h("textarea", { id: "json", rows: 4, spellcheck: "false" });
  const flash = (b, t) => { const o = b.textContent; b.textContent = t; setTimeout(() => b.textContent = o, 1200); };
  const copy = h("button", {}, "Copy settings JSON"), paste = h("button", { class: "ghost" }, "Paste settings");
  copy.onclick = async () => { ta.value = JSON.stringify(S); try { await navigator.clipboard.writeText(ta.value); flash(copy, "Copied ✓"); } catch { ta.select(); document.execCommand("copy"); flash(copy, "Copied ✓"); } };
  paste.onclick = async () => {
    let txt = "";
    try { txt = await navigator.clipboard.readText(); } catch {}
    if (!txt.trim().startsWith("{")) txt = ta.value;
    try { Object.assign(S, JSON.parse(txt)); syncUI(); render(); flash(paste, "Applied ✓"); } catch { flash(paste, "Not valid JSON"); }
  };
  const exportCmd = "node assets/icon/generator/render.mjs out play --settings settings.json --icns";
  const dlj = h("button", { class: "ghost" }, "Download settings.json");
  dlj.onclick = () => { const a = h("a", { href: "data:application/json," + encodeURIComponent(JSON.stringify(S, null, 1)), download: "settings.json" }); a.click(); };
  const cmdBtn = h("button", { class: "ghost" }, "Copy export command");
  cmdBtn.onclick = async () => { try { await navigator.clipboard.writeText(exportCmd); flash(cmdBtn, "Copied ✓"); } catch { flash(cmdBtn, "see below"); } };
  root.append(h("details", { open: "" }, h("summary", {}, "Settings & export"), h("div", { class: "body" }, ta,
    h("div", { class: "btns" }, copy, paste), h("div", { class: "btns" }, dlj, cmdBtn),
    h("p", { class: "note" }, "Export .icns: save settings.json in the repo root (Download settings.json, or paste the JSON into it), then run:"),
    h("pre", {}, exportCmd),
    h("p", { class: "note" }, "That renders the 1024px PNG and builds out/alpha-icon-play-playground.icns (sips + iconutil). It never touches assets/icon/Alpha.icns."))));
  syncUI();
  document.getElementById("dl").onclick = () => { const a = h("a", { href: window.__png, download: `alpha-icon-${S.font}-${S.weight}.png` }); a.click(); };
  document.getElementById("reset").onclick = () => { Object.assign(S, JSON.parse(JSON.stringify(DEFAULTS))); syncUI(); render(); };
  const stage = document.getElementById("stage");
  document.querySelectorAll("[data-bd]").forEach(b => b.onclick = () => { stage.dataset.bd = b.dataset.bd; document.querySelectorAll("[data-bd]").forEach(x => x.classList.toggle("on", x === b)); });
  document.getElementById("json").onchange = () => {};
}
setInterval(() => { const t = document.getElementById("json"); if (t && document.activeElement !== t) t.value = JSON.stringify(S); }, 1500);
