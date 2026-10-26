// Interactive variant 2 (rosewood oval): pick a font, size, centering and nudge, re-render live.
// Fonts: the app's EB Garamond (local) plus static Google fonts via fontsource on jsDelivr.
const FONTS = [
  ["EB Garamond (app)", "local", [400, 500]],
  ["Playfair Display", "playfair-display", [400, 500, 600, 700, 800, 900]],
  ["Cormorant Garamond", "cormorant-garamond", [400, 500, 600, 700]],
  ["Lora", "lora", [400, 500, 600, 700]],
  ["Libre Baskerville", "libre-baskerville", [400, 700]],
  ["Libre Caslon Text", "libre-caslon-text", [400, 700]],
  ["DM Serif Display", "dm-serif-display", [400]],
  ["Fraunces", "fraunces", [400, 500, 600, 700, 800]],
  ["Newsreader", "newsreader", [400, 500, 600, 700]],
  ["Crimson Pro", "crimson-pro", [400, 500, 600, 700]],
  ["Bodoni Moda", "bodoni-moda", [400, 500, 600, 700]],
  ["Instrument Serif", "instrument-serif", [400]],
  ["Source Serif 4", "source-serif-4", [400, 500, 600, 700]],
  ["Spectral", "spectral", [400, 500, 600, 700]],
  ["Young Serif", "young-serif", [400]],
  ["Gloock", "gloock", [400]],
];
const LOCAL = { 400: "../../fonts/EBGaramond-Regular.ttf", 500: "../../fonts/EBGaramond-Medium.ttf" };

const S = { font: 1, weight: 700, char: "a", size: 560, center: "blend", dx: 0, dy: 0, angle: 83, seed: 41, ring: true };
let busy = false, again = false, glyph = null, lastBlob = null;
const fontCache = {};

function flatten(cmds, steps = 14) {
  const out = []; let cur = null, px = 0, py = 0;
  for (const c of cmds) {
    if (c.type === "M") { cur = [[c.x, c.y]]; out.push(cur); px = c.x; py = c.y; }
    else if (c.type === "L") { cur.push([c.x, c.y]); px = c.x; py = c.y; }
    else if (c.type === "Q") { for (let i = 1; i <= steps; i++) { const t = i / steps, u = 1 - t; cur.push([u*u*px + 2*u*t*c.x1 + t*t*c.x, u*u*py + 2*u*t*c.y1 + t*t*c.y]); } px = c.x; py = c.y; }
    else if (c.type === "C") { for (let i = 1; i <= steps; i++) { const t = i / steps, u = 1 - t; cur.push([u*u*u*px + 3*u*u*t*c.x1 + 3*u*t*t*c.x2 + t*t*t*c.x, u*u*u*py + 3*u*u*t*c.y1 + 3*u*t*t*c.y2 + t*t*t*c.y]); } px = c.x; py = c.y; }
  }
  return out;
}

function fontUrl() {
  const [, id] = FONTS[S.font];
  return id === "local" ? LOCAL[S.weight] : `https://cdn.jsdelivr.net/npm/@fontsource/${id}/files/${id}-latin-${S.weight}-normal.woff`;
}
function fetchFont(url) {
  return fontCache[url] ??= new Promise((res, rej) => opentype.load(url, (e, f) => e ? rej(e) : res(f)));
}

// Area centroid of a polygon.
function centroid(poly) {
  let a = 0, cx = 0, cy = 0;
  for (let i = 0; i < poly.length; i++) {
    const [x0, y0] = poly[i], [x1, y1] = poly[(i + 1) % poly.length], k = x0 * y1 - x1 * y0;
    a += k; cx += (x0 + x1) * k; cy += (y0 + y1) * k;
  }
  return [cx / (3 * a), cy / (3 * a), a / 2];
}

// Place the loaded glyph: scale so max(width, height) = S.size, centre by bbox / area centroid / a blend.
function place(contours) {
  const all = contours.flat();
  const xs = all.map(p => p[0]), ys = all.map(p => p[1]);
  const x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys);
  const s = S.size / Math.max(x1 - x0, y1 - y0);
  // area centroid of the filled shape (outer minus holes)
  let A = 0, CX = 0, CY = 0;
  for (const c of contours) { const [cx, cy, a] = centroid(c); A += a; CX += cx * a; CY += cy * a; }
  CX /= A; CY /= A;
  const bx = (x0 + x1) / 2, by = (y0 + y1) / 2;
  const t = { bbox: 0, blend: 0.5, centroid: 1 }[S.center];
  const mx = bx + (CX - bx) * t, my = by + (CY - by) * t;
  const placed = contours.map(c => c.map(p => [600 + S.dx + (p[0] - mx) * s, 600 + S.dy + (p[1] - my) * s]));
  placed.sort((a, b) => Math.abs(polyArea(b)) - Math.abs(polyArea(a)));
  return { outer: placed[0], holes: placed.slice(1) };
}

function drawIcon() {
  darkGround();
  roseField("#4f3228", "#1f1613", 8);
  if (S.ring) bandedOval(600, 600, 500, 525, 482, 507, 56);
  const field = ellipsePoints(600, 600, 478, 503, 0, 120);
  shadedPiece(field, "#3d281f", "#1c1411", 0.05, 5, "#1c1411", 0.4);
  grainInShape(field, -4, 8, rgba("#150f0d", 125), 0.26, 4, 3.3);
  grainInShape(field, -4, 22, rgba("#9a6a4a", 36), 0.2, 3, 1.1);
  inlayGlyph(place(glyph), SATIN, S.angle, { ...ROSE, base: "#4b3026" }, 10, { halo: 8, depth: 9 });
}

function squirclePath(ctx, cx, cy, half, n = 5) {
  ctx.beginPath();
  for (let i = 0; i <= 360; i++) {
    const t = i * Math.PI / 180, c = Math.cos(t), s = Math.sin(t);
    const x = cx + half * Math.sign(c) * Math.pow(Math.abs(c), 2 / n), y = cy + half * Math.sign(s) * Math.pow(Math.abs(s), 2 / n);
    i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
  }
  ctx.closePath();
}

async function compose() {
  const src = document.querySelector("canvas:not(#out)");
  const out = document.getElementById("out"), ctx = out.getContext("2d");
  ctx.clearRect(0, 0, 1024, 1024);
  ctx.imageSmoothingQuality = "high";
  ctx.save(); ctx.shadowColor = "rgba(0,0,0,.30)"; ctx.shadowBlur = 24; ctx.shadowOffsetY = 10;
  squirclePath(ctx, 512, 512, 412); ctx.fillStyle = "#000"; ctx.fill(); ctx.restore();
  ctx.save(); squirclePath(ctx, 512, 512, 412); ctx.clip(); ctx.drawImage(src, 100, 100, 824, 824); ctx.restore();
  squirclePath(ctx, 512, 512, 411); ctx.lineWidth = 2; ctx.strokeStyle = "rgba(0,0,0,.22)"; ctx.stroke();
  const url = out.toDataURL("image/png");
  for (const id of ["p128", "p64", "p32"]) document.getElementById(id).src = url;
  lastBlob = url;
}

async function setup() {
  createCanvas(1200, 1200, WEBGL);
  brush.scaleBrushes(6);
  angleMode(DEGREES);
  noLoop();
  buildUI();
  render();
}
function draw() {
  if (!glyph) return;
  randomSeed(S.seed); noiseSeed(S.seed);
  background("#30241e");
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
    status.textContent = "rendering…";
    await new Promise(r => setTimeout(r, 30));
    const t0 = performance.now();
    await redraw();
    await compose();
    if (new URLSearchParams(location.search).get('auto')) { const b = await (await fetch(lastBlob)).blob(); await fetch('/save?name=play', { method: 'POST', body: b }); window.__done = true; }
    status.textContent = `${FONTS[S.font][0]} ${S.weight} · ${((performance.now() - t0) / 1000).toFixed(1)}s`;
  } catch (e) {
    status.textContent = "couldn't load that font/weight: " + (e.message || e);
  }
  busy = false;
  if (again) { again = false; render(); }
}

let timer;
const later = () => { clearTimeout(timer); timer = setTimeout(render, 350); };

function buildUI() {
  const $ = id => document.getElementById(id);
  const fsel = $("font"), wsel = $("weight");
  FONTS.forEach((f, i) => fsel.add(new Option(f[0], i)));
  const fillWeights = () => {
    wsel.innerHTML = "";
    FONTS[S.font][2].forEach(w => wsel.add(new Option(w, w)));
    if (!FONTS[S.font][2].includes(S.weight)) S.weight = FONTS[S.font][2].slice(-1)[0] >= 700 ? 700 : FONTS[S.font][2][0];
    wsel.value = S.weight;
  };
  fsel.value = S.font; fillWeights();
  fsel.onchange = () => { S.font = +fsel.value; fillWeights(); render(); };
  wsel.onchange = () => { S.weight = +wsel.value; render(); };
  const bind = (id, key, num = true) => { const el = $(id), out = $(id + "-v"); el.oninput = () => { S[key] = num ? +el.value : el.value; if (out) out.textContent = el.value; later(); }; };
  bind("size", "size"); bind("dx", "dx"); bind("dy", "dy"); bind("angle", "angle"); bind("seed", "seed");
  $("char").oninput = e => { S.char = e.target.value.slice(0, 1) || "a"; later(); };
  $("center").onchange = e => { S.center = e.target.value; render(); };
  $("ring").onchange = e => { S.ring = e.target.checked; render(); };
  $("reset").onclick = () => { for (const [k, v] of [["size", 560], ["dx", 0], ["dy", 0], ["angle", 83], ["seed", 41]]) { S[k] = v; $(k).value = v; $(k + "-v").textContent = v; } render(); };
  $("dl").onclick = () => { const a = document.createElement("a"); a.href = lastBlob; a.download = `alpha-icon-v2-${FONTS[S.font][1]}-${S.weight}.png`; a.click(); };
  document.addEventListener("keydown", e => {
    if (e.target.tagName === "INPUT" || e.target.tagName === "SELECT") return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") { S.font = (S.font + (e.key === "ArrowDown" ? 1 : FONTS.length - 1)) % FONTS.length; fsel.value = S.font; fillWeights(); render(); e.preventDefault(); }
  });
}
