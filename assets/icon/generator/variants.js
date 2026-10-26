// The six icon variants. Each draws full-bleed art on the 1200x1200 canvas (sketch coordinates).

const A = "regular:a";

// 1. serif 'a' as a single inlaid veneer piece on the perspective parquet.
function variant1() {
  darkGround();
  drawParquet();
  const g = placeGlyph(A, 600, 610, 760);
  inlayGlyph(g, SATIN, 83, ROSE, -20, { halo: 9, depth: 11 });
}

// 2. satin 'a' inlaid into a dark rosewood tile, ringed by the banded oval.
function variant2() {
  darkGround();
  roseField("#4f3228", "#1f1613", 8);
  bandedOval(600, 600, 500, 525, 482, 507, 56);
  const field = ellipsePoints(600, 600, 478, 503, 0, 120);
  shadedPiece(field, "#3d281f", "#1c1411", 0.05, 5, "#1c1411", 0.4);
  grainInShape(field, -4, 8, rgba("#150f0d", 125), 0.26, 4, 3.3);
  grainInShape(field, -4, 22, rgba("#9a6a4a", 36), 0.2, 3, 1.1);
  const g = placeGlyph(A, 600, 612, 560);
  inlayGlyph(g, SATIN, 83, { ...ROSE, base: "#4b3026" }, 10, { halo: 8, depth: 9 });
}

// 3. 'a' built from several veneer pieces, each with its own wood tone and grain angle.
function variant3() {
  darkGround();
  roseField("#33251f", "#17100d", 3, 0.04);
  brush.noFill(); brush.noWash();
  brush.set("rotring", rgba("#c9a56f", 80), 0.3);
  outlinePoly([[105, 105], [1095, 105], [1095, 1095], [105, 1095]]);
  const g = placeGlyph(A, 600, 610, 740);
  const xs = g.outer.map(p => p[0]), ys = g.outer.map(p => p[1]);
  const x0 = Math.min(...xs), x1 = Math.max(...xs);
  const tilt = -22 * Math.PI / 180, dir = [Math.cos(tilt), Math.sin(tilt)], nrm = [-dir[1], dir[0]];
  // 5 diagonal bands across the glyph, split at irregular offsets
  const cuts = [-0.34, -0.13, 0.06, 0.26];
  const span = x1 - x0, cx = (x0 + x1) / 2, cy = (Math.min(...ys) + Math.max(...ys)) / 2;
  const edges = [-1e4, ...cuts.map(c => c * span), 1e4];
  const woods = [
    { ...SATIN, base: "#e3cb9a", edge: "#8f6a47" },
    { ...SATIN, base: "#d3b078", edge: "#7d5a3b" },
    { ...SATIN, base: "#ead6a8", edge: "#947050" },
    { ...SATIN, base: "#c8a367", edge: "#74533a" },
    { ...SATIN, base: "#dcc08a", edge: "#85613f" },
  ];
  const angles = [83, -28, 48, -68, 12];
  for (let i = 0; i < 5; i++) {
    const a = edges[i], b = edges[i + 1];
    const q = [a, b].map(o => o).flatMap((o, k) => k === 0
      ? [[cx + nrm[0] * o - dir[0] * 2000, cy + nrm[1] * o - dir[1] * 2000], [cx + nrm[0] * o + dir[0] * 2000, cy + nrm[1] * o + dir[1] * 2000]]
      : [[cx + nrm[0] * o + dir[0] * 2000, cy + nrm[1] * o + dir[1] * 2000], [cx + nrm[0] * o - dir[0] * 2000, cy + nrm[1] * o - dir[1] * 2000]]);
    const res = polygonClipping.intersection([g.outer], [q]);
    res.forEach((poly, j) => {
      const ring = poly[0].slice(0, -1);
      if (Math.abs(polyArea(ring)) < 200) return;
      veneerPiece(ring, woods[i], { angle: angles[i], phase: i * 1.9 + j }, { halo: i === 0 ? 8 : 0, depth: 6, steps: 4, seam: "#2a1d17" });
    });
  }
  // keep one continuous outer halo-free look: re-stroke the glyph outline
  brush.noFill(); brush.noWash();
  brush.set("rotring", rgba("#241915", 200), 0.5);
  outlinePoly(g.outer);
  for (const h of g.holes) counterPiece(h, ROSE, -30, 0.5);
}

// 4. open book in veneer with an 'a' inlay on the right page.
function variant4() {
  darkGround();
  roseField("#5a372b", "#1f1612", 2, 0.04);
  const mirror = pts => pts.map(p => [1200 - p[0], p[1]]);
  const left = closedBezier([600, 372], [
    [[520, 330], [340, 316], [172, 346]],
    [[166, 520], [166, 700], [172, 830]],
    [[340, 800], [520, 820], [600, 872]],
    [[600, 700], [600, 540], [600, 372]],
  ], 16);
  const right = mirror(left);
  // cover peeking out around the pages + stacked page edges
  const cover = closedBezier([600, 400], [
    [[500, 340], [300, 322], [120, 366]],
    [[112, 540], [112, 760], [120, 880]],
    [[300, 842], [500, 862], [600, 920]],
    [[700, 862], [900, 842], [1080, 880]],
    [[1088, 760], [1088, 540], [1080, 366]],
    [[900, 322], [700, 340], [600, 400]],
  ], 16);
  shadedPiece(cover, "#6d4336", "#241915", 0.05, 5, "#1c1411", 0.5);
  grainInShape(cover, 4, 8, rgba("#241915", 120), 0.27, 4, 0.2);
  for (const dy of [26, 14]) {
    const lay = (pts) => pts.map(p => [p[0], p[1] + dy]);
    drawPoly(lay(left), dy === 26 ? "#cfb68a" : "#e0c99c", "#4a3326", 0.2);
    drawPoly(lay(right), dy === 26 ? "#cfb68a" : "#e0c99c", "#4a3326", 0.2);
  }
  shadedPiece(left, "#e6cf9f", "#9a7650", 0.05, 6, "#241915", 0.46);
  grainInShape(left, 84, 8, rgba("#967747", 50), 0.23, 2.4, 0.4);
  grainInShape(left, 84, 22, rgba("#fff0c2", 60), 0.22, 2.4, 2.2);
  shadedPiece(right, "#e6cf9f", "#9a7650", 0.05, 6, "#241915", 0.46);
  grainInShape(right, 96, 8, rgba("#967747", 50), 0.23, 2.4, 1.4);
  grainInShape(right, 96, 22, rgba("#fff0c2", 60), 0.22, 2.4, 0.2);
  // ruled hairlines on the left page
  brush.set("rotring", rgba("#6a4a32", 140), 0.2);
  for (let i = 0; i < 6; i++) {
    const y = 480 + i * 56;
    brush.spline([[230, y + 6], [330, y - 6 + i], [430, y + 4], [540, y - 2]], 0.3);
  }
  // spine seam
  brush.noFill(); brush.noWash();
  brush.set("rotring", rgba("#241915", 230), 0.6);
  brush.spline([[600, 372], [602, 540], [600, 700], [600, 872]], 0.3);
  const g = placeGlyph(A, 890, 618, 310);
  inlayGlyph(g, { ...ROSE, base: "#6a4234", edge: "#241915" }, -24, { ...SATIN }, 80, { halo: 5, depth: 5, steps: 4, seam: "#3a281f" });
}

// 5. minimal: flat rosewood tile, satin 'a', one seam line. Built for 32px legibility.
function variant5() {
  darkGround();
  const rect = [[0, 0], [1200, 0], [1200, 1200], [0, 1200]];
  shadedPiece(rect, "#74483a", "#4f2f26", 0.03, 3, "#4f2f26", 0.3);
  grainInShape(rect, 3, 16, rgba("#2b1b16", 55), 0.24, 4, 0.7);
  const g = placeGlyph(A, 600, 560, 820);
  const baseline = Math.max(...g.outer.map(p => p[1]));
  brush.noFill(); brush.noWash();
  brush.set("rotring", rgba("#1c1411", 255), 0.9);
  brush.spline([[0, baseline], [400, baseline + 1], [800, baseline - 1], [1200, baseline]], 0.2);
  brush.set("rotring", rgba("#c9946a", 90), 0.28);
  brush.spline([[0, baseline + 8], [400, baseline + 9], [800, baseline + 7], [1200, baseline + 8]], 0.2);
  veneerPiece(g.outer, SATIN, { angle: 83, phase: 0.4 }, { halo: 6, depth: 8, steps: 4, holes: g.holes });
  for (const h of g.holes) {
    drawPoly(h, "#74483a", "#241915", 0.4);
    grainInShape(h, 3, 16, rgba("#2b1b16", 55), 0.24, 4, 0.7);
  }
}

// 6. free variation: sunburst rosette in alternating woods, banded ring, 'a' on an ebony roundel.
function variant6() {
  darkGround();
  const n = 32, R = 1000, cx = 600, cy = 600;
  const satin = ["#c5a16d", "#b99662", "#cfad77", "#b88f5e"];
  const rose = ["#784b3b", "#6d4336", "#825140", "#734739"];
  for (let i = 0; i < n; i++) {
    const a0 = i * 360 / n - 90 - 360 / n / 2, a1 = a0 + 360 / n;
    const pts = [[cx, cy]];
    for (let k = 0; k <= 4; k++) pts.push(ovalPoint(cx, cy, R, R, lerp(a0, a1, k / 4)));
    const light = i % 2 === 0;
    const base = (light ? satin : rose)[Math.floor(i / 2) % 4];
    drawPoly(pts, mixColor(base, "#2d201a", 0.34), "#35251e", 0.24);
    drawPoly(scalePoly(pts, 0.985), base, null);
    const mid = (a0 + a1) / 2;
    grainInShape(scalePoly(pts, 0.985), mid, 8, light ? rgba("#6e4e31", 68) : rgba("#321f1a", 85), 0.2, 2.2, i * 1.3);
    grainInShape(scalePoly(pts, 0.985), mid, 22, light ? rgba("#ead09b", 44) : rgba("#bd8260", 35), 0.18, 2, i * 0.7);
  }
  bandedOval(cx, cy, 440, 440, 420, 420, 48);
  const disc = ellipsePoints(cx, cy, 418, 418, 0, 120);
  shadedPiece(disc, "#3a271f", "#171110", 0.06, 6, "#171110", 0.5);
  grainInShape(disc, 20, 8, rgba("#120d0b", 135), 0.25, 3, 1.4);
  grainInShape(disc, 20, 24, rgba("#8a6144", 30), 0.2, 3, 0.2);
  const g = placeGlyph(A, 600, 608, 470);
  inlayGlyph(g, SATIN, 83, { ...ROSE, base: "#3a271f", edge: "#171110" }, 20, { halo: 6, depth: 7, seam: "#120d0b" });
}

const VARIANTS = {
  v1: { fn: variant1, name: "parquet-inlay" },
  v2: { fn: variant2, name: "rosewood-oval" },
  v3: { fn: variant3, name: "pieced-grain" },
  v4: { fn: variant4, name: "open-book" },
  v5: { fn: variant5, name: "minimal-seam" },
  v6: { fn: variant6, name: "sunburst-rosette" },
};
