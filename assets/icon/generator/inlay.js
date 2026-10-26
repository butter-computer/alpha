// Glyph inlay helpers built on the marquetry-veneer helpers (helpers.js).

const SATIN = { base: "#dfc694", edge: "#8a6544", dark: rgba0("#967747", 55), light: rgba0("#fff0c2", 65) };
const ROSE = { base: "#784b3b", edge: "#291d19", dark: rgba0("#241915", 145), light: rgba0("#c09260", 44) };

// rgba() needs p5's color(), which only exists after p5 boots, so defer it.
function rgba0(hex, alpha) { return () => rgba(hex, alpha); }
function col(c) { return typeof c === "function" ? c() : c; }

// Glyph outline placed so its bounding box is `height` tall and centered on (cx, cy).
// Returns { outer, holes } in canvas coordinates.
function placeGlyph(key, cx, cy, height) {
  const contours = window.GLYPHS[key];
  const all = contours.flat();
  const xs = all.map(p => p[0]), ys = all.map(p => p[1]);
  const minX = Math.min(...xs), maxX = Math.max(...xs);
  const minY = Math.min(...ys), maxY = Math.max(...ys);
  const s = height / (maxY - minY);
  const mx = (minX + maxX) / 2, my = (minY + maxY) / 2;
  const placed = contours.map(c => c.map(p => [cx + (p[0] - mx) * s, cy + (p[1] - my) * s]));
  placed.sort((a, b) => Math.abs(polyArea(b)) - Math.abs(polyArea(a)));
  return { outer: placed[0], holes: placed.slice(1) };
}

// Offset a polygon along its vertex normals: d > 0 shrinks, d < 0 grows. Miter-limited.
function insetPoly(points, d) {
  const pts = points.filter((p, i) => {
    const q = points[(i + 1) % points.length];
    return Math.hypot(p[0] - q[0], p[1] - q[1]) > 1e-6;
  });
  const n = pts.length;
  const norm = (a, b) => {
    const dx = b[0] - a[0], dy = b[1] - a[1], l = Math.hypot(dx, dy) || 1;
    return [dy / l, -dx / l];
  };
  // pick the normal side that points into the polygon
  let sign = 1;
  for (let i = 0; i < n; i++) {
    const a = pts[i], b = pts[(i + 1) % n];
    if (Math.hypot(b[0] - a[0], b[1] - a[1]) < 2) continue;
    const nn = norm(a, b);
    const m = [(a[0] + b[0]) / 2 + nn[0] * 0.5, (a[1] + b[1]) / 2 + nn[1] * 0.5];
    sign = pointInPolygon(m, pts) ? 1 : -1;
    break;
  }
  return pts.map((p, i) => {
    const n1 = norm(pts[(i + n - 1) % n], p), n2 = norm(p, pts[(i + 1) % n]);
    let bx = n1[0] + n2[0], by = n1[1] + n2[1];
    const bl = Math.hypot(bx, by) || 1;
    bx /= bl; by /= bl;
    const k = Math.min(1 / Math.max(bx * n1[0] + by * n1[1], 0.5), 2);
    return [p[0] + sign * bx * d * k, p[1] + sign * by * d * k];
  });
}

// shadedPiece with a true inward offset (suits thin letterforms better than scaling).
function shadedInset(points, base, edge, depth, steps, seam, seamWeight) {
  drawPoly(points, edge, null);
  for (let i = 1; i <= steps; i++) {
    const t = i / steps;
    drawPoly(insetPoly(points, depth * t), mixColor(edge, base, Math.pow(t, 0.72)), null);
  }
  brush.noFill();
  brush.noWash();
  brush.set("rotring", seam, seamWeight);
  outlinePoly(points);
}

// One veneer piece (a polygon + optional holes) with seam halo, shading and wavy grain.
// wood: { base, edge, dark, light }, grain: { angle, phase }
function veneerPiece(outer, wood, grain, opts = {}) {
  const { depth = 10, steps = 5, halo = 7, seam = "#241915", holes = [] } = opts;
  if (halo > 0) drawPoly(insetPoly(outer, -halo), seam, null);
  shadedInset(outer, wood.base, wood.edge, depth, steps, seam, 0.46);
  const a = grain.angle, ph = grain.phase ?? 0;
  grainInShape(outer, a, 7, col(wood.dark), 0.27, 3.2, ph, holes);
  grainInShape(outer, a, 19, col(wood.light), 0.23, 3.0, ph * 1.7 + 0.8, holes);
}

// The counter of a letter: its own small inlay.
function counterPiece(hole, wood, angle, phase = 0) {
  shadedPiece(hole, wood.base, wood.edge, 0.07, 4, "#241915", 0.42);
  grainInShape(hole, angle, 6.5, col(wood.dark), 0.26, 2.4, phase);
  grainInShape(hole, angle, 18, col(wood.light), 0.2, 2.2, phase + 1);
}

// Glyph as one inlaid piece.
function inlayGlyph(g, wood, grainAngle, counterWood, counterAngle, opts = {}) {
  veneerPiece(g.outer, wood, { angle: grainAngle, phase: 0.4 }, { ...opts, holes: g.holes });
  for (const h of g.holes) counterPiece(h, counterWood, counterAngle, 1.3);
}

// Banded inlaid ring (the oval border from the sketch), parameterised.
function bandedOval(cx, cy, outerX, outerY, innerX, innerY, count) {
  for (let i = 0; i < count; i++) {
    const a0 = i * 360 / count, a1 = (i + 1) * 360 / count;
    const pts = [];
    for (let k = 0; k <= 3; k++) pts.push(ovalPoint(cx, cy, outerX, outerY, lerp(a0, a1, k / 3)));
    for (let k = 3; k >= 0; k--) pts.push(ovalPoint(cx, cy, innerX, innerY, lerp(a0, a1, k / 3)));
    const light = (Math.floor(i / 2) % 2) === 0;
    const base = light ? ["#b18a58", "#a98254"][i % 2] : ["#694235", "#71483a"][i % 2];
    drawPoly(pts, mixColor(base, "#281d18", 0.3), "#2b201a", 0.24);
    drawPoly(scalePoly(pts, 0.87), base, null);
    const mid = [];
    for (let k = 0; k <= 4; k++) mid.push(ovalPoint(cx, cy, (outerX + innerX) / 2, (outerY + innerY) / 2, lerp(a0 + 0.25, a1 - 0.25, k / 4)));
    brush.set("rotring", light ? rgba("#5d4028", 70) : rgba("#d0a06b", 40), 0.18);
    brush.spline(mid, 0.25);
  }
  brush.noFill();
  brush.noWash();
  brush.set("rotring", rgba("#241b17", 190), 0.42);
  brush.spline(ellipsePoints(cx, cy, outerX, outerY, 0, 144).concat([ovalPoint(cx, cy, outerX, outerY, 0)]), 0.2);
  brush.set("rotring", rgba("#d0aa72", 85), 0.26);
  brush.spline(ellipsePoints(cx, cy, innerX, innerY, 0, 144).concat([ovalPoint(cx, cy, innerX, innerY, 0)]), 0.2);
}

// Full-bleed dark ground + a rosewood field piece.
function darkGround() {
  brush.noStroke();
  brush.wash("#34261f", 255);
  brush.rect(0, 0, 1200, 1200);
  brush.noWash();
}
function roseField(base = "#5e3a2e", edge = "#241915", angle = 6, inset = 0.045) {
  const rect = [[0, 0], [1200, 0], [1200, 1200], [0, 1200]];
  shadedPiece(rect, base, edge, inset, 5, "#241915", 0.4);
  grainInShape(rect, angle, 9, rgba("#1e1512", 120), 0.27, 5, 0.9);
  grainInShape(rect, angle, 23, rgba("#b07a55", 40), 0.22, 4, 2.4);
}
