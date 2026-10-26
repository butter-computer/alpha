// Perspective parquet ground, verbatim from the 'marquetry veneer' sketch.
function projectTable(x, y) {
  const d = y + 14;
  return [600 + x * 1600 / d, -300 + 10000 / d];
}

function drawParquet() {
  const satin = (window.PARQUET && PARQUET.satin) || ["#c5a16d", "#b99662", "#cfad77", "#b88f5e"]; // PARQUET overrides: playground palettes
  const rose = (window.PARQUET && PARQUET.rose) || ["#784b3b", "#6d4336", "#825140", "#734739"];

  for (let y = -9; y <= 25; y++) {
    const d = y + 14;
    const xBound = Math.ceil(720 * Math.max(d, 4) / 1600) + 4;

    for (let x = -xBound; x <= xBound; x++) {
      const s = ((x + y) % 4 + 4) % 4;
      if (s !== 0 && s !== 2) continue;

      const horizontal = s === 0;
      const world = horizontal
        ? [[x, y], [x + 2, y], [x + 2, y + 1], [x, y + 1]]
        : [[x, y], [x + 1, y], [x + 1, y + 2], [x, y + 2]];

      const pts = world.map(p => projectTable(p[0], p[1]));
      const xs = pts.map(p => p[0]);
      const ys = pts.map(p => p[1]);

      if (Math.max(...xs) < -30 || Math.min(...xs) > 1230 ||
          Math.max(...ys) < -30 || Math.min(...ys) > 1230) continue;

      const area = Math.abs(polyArea(pts));
      if (area < 2) continue;

      const palette = horizontal ? satin : rose;
      const base = palette[Math.floor(random(palette.length))];
      const edge = mixColor(base, "#2d201a", 0.34);

      drawPoly(pts, edge, "#35251e", area < 80 ? 0.16 : 0.23);
      if (area > 32) {
        drawPoly(scalePoly(pts, 0.968), base, null);
      }

      if (area > 42) {
        const darkGrain = horizontal ? rgba("#6e4e31", 68) : rgba("#321f1a", 85);
        const lightGrain = horizontal ? rgba("#ead09b", 44) : rgba("#bd8260", 35);
        brush.set("rotring", darkGrain, area < 180 ? 0.17 : 0.22);

        const count = area > 900 ? 3 : area > 180 ? 2 : 1;
        const phase = random(100);

        for (let g = 0; g < count; g++) {
          const f = (g + 1) / (count + 1);
          const line = [];

          for (let k = 0; k <= 12; k++) {
            const t = k / 12;
            let u, v;

            if (horizontal) {
              u = x + 0.07 + 1.86 * t;
              v = y + f + 0.025 * Math.sin(t * 8 + phase + g);
            } else {
              u = x + f + 0.025 * Math.sin(t * 8 + phase + g);
              v = y + 0.07 + 1.86 * t;
            }
            line.push(projectTable(u, v));
          }
          brush.spline(line, 0.28);
        }

        if (area > 500) {
          brush.set("rotring", lightGrain, 0.18);
          const line = [];
          for (let k = 0; k <= 12; k++) {
            const t = k / 12;
            const u = horizontal ? x + 0.1 + 1.8 * t : x + 0.72;
            const v = horizontal ? y + 0.72 : y + 0.1 + 1.8 * t;
            line.push(projectTable(u, v));
          }
          brush.spline(line, 0.25);
        }
      }
    }
  }
}
