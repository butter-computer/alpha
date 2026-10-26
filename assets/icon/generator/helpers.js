// Helpers copied verbatim from the 'marquetry veneer' p5.brush sketch (grainInShape gains an optional holes arg).
function shadedPiece(points, base, edge, inset, steps, seam, seamWeight) {
  drawPoly(points, edge, null);

  for (let i = 1; i <= steps; i++) {
    const t = i / steps;
    const c = mixColor(edge, base, Math.pow(t, 0.72));
    drawPoly(scalePoly(points, 1 - inset * t), c, null);
  }

  brush.noFill();
  brush.noWash();
  brush.set("rotring", seam, seamWeight);
  outlinePoly(points);
}

function drawPoly(points, fillColor, seamColor, seamWeight = 0.3) {
  brush.noFill();
  brush.wash(fillColor, 255);

  if (seamColor) {
    brush.set("rotring", seamColor, seamWeight);
  } else {
    brush.noStroke();
  }

  brush.beginShape(0);
  for (const p of points) brush.vertex(p[0], p[1]);
  brush.endShape(true);
  brush.noWash();
}

function outlinePoly(points) {
  brush.beginShape(0);
  for (const p of points) brush.vertex(p[0], p[1]);
  brush.endShape(true);
}

function grainInShape(points, angle, spacing, strokeColor, weight, amplitude, phase, holes = []) {
  const xs = points.map(p => p[0]);
  const ys = points.map(p => p[1]);
  const minX = Math.min(...xs), maxX = Math.max(...xs);
  const minY = Math.min(...ys), maxY = Math.max(...ys);
  const cx = (minX + maxX) / 2;
  const cy = (minY + maxY) / 2;
  const diag = Math.hypot(maxX - minX, maxY - minY) * 0.72;
  const a = angle * Math.PI / 180;
  const ca = Math.cos(a), sa = Math.sin(a);

  brush.set("rotring", strokeColor, weight);

  for (let off = -diag; off <= diag; off += spacing) {
    let segment = [];

    for (let t = -diag; t <= diag; t += 8) {
      const wave = amplitude * Math.sin(t * 0.026 + off * 0.055 + phase)
        + amplitude * 0.38 * Math.sin(t * 0.063 - off * 0.021 + phase * 1.7);
      const normal = off + wave;
      const x = cx + ca * t - sa * normal;
      const y = cy + sa * t + ca * normal;

      if (pointInPolygon([x, y], points) && !holes.some(h => pointInPolygon([x, y], h))) {
        segment.push([x, y]);
      } else {
        if (segment.length > 1) brush.spline(segment, 0.32);
        segment = [];
      }
    }

    if (segment.length > 1) brush.spline(segment, 0.32);
  }
}

function pointInPolygon(p, poly) {
  let inside = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const xi = poly[i][0], yi = poly[i][1];
    const xj = poly[j][0], yj = poly[j][1];
    const intersect = ((yi > p[1]) !== (yj > p[1])) &&
      (p[0] < (xj - xi) * (p[1] - yi) / ((yj - yi) || 0.00001) + xi);
    if (intersect) inside = !inside;
  }
  return inside;
}

function scalePoly(points, amount) {
  let cx = 0, cy = 0;
  for (const p of points) {
    cx += p[0];
    cy += p[1];
  }
  cx /= points.length;
  cy /= points.length;
  return points.map(p => [
    cx + (p[0] - cx) * amount,
    cy + (p[1] - cy) * amount
  ]);
}

function polyArea(points) {
  let area = 0;
  for (let i = 0; i < points.length; i++) {
    const j = (i + 1) % points.length;
    area += points[i][0] * points[j][1] - points[j][0] * points[i][1];
  }
  return area / 2;
}

function closedBezier(start, segments, steps) {
  const points = [start];
  let current = start;

  for (const segment of segments) {
    const curve = sampleCubic(current, segment[0], segment[1], segment[2], steps);
    points.push(...curve.slice(1));
    current = segment[2];
  }
  return points;
}

function sampleCubic(p0, p1, p2, p3, steps) {
  const points = [];
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const u = 1 - t;
    points.push([
      u * u * u * p0[0] + 3 * u * u * t * p1[0] +
        3 * u * t * t * p2[0] + t * t * t * p3[0],
      u * u * u * p0[1] + 3 * u * u * t * p1[1] +
        3 * u * t * t * p2[1] + t * t * t * p3[1]
    ]);
  }
  return points;
}

function sampleQuadratic(p0, p1, p2, steps) {
  const points = [];
  for (let i = 0; i <= steps; i++) {
    points.push(quadPoint(p0, p1, p2, i / steps));
  }
  return points;
}

function quadPoint(p0, p1, p2, t) {
  const u = 1 - t;
  return [
    u * u * p0[0] + 2 * u * t * p1[0] + t * t * p2[0],
    u * u * p0[1] + 2 * u * t * p1[1] + t * t * p2[1]
  ];
}

function ellipsePoints(cx, cy, rx, ry, rotation, count) {
  const points = [];
  const r = rotation * Math.PI / 180;
  const cr = Math.cos(r), sr = Math.sin(r);

  for (let i = 0; i < count; i++) {
    const a = i * Math.PI * 2 / count;
    const x = Math.cos(a) * rx;
    const y = Math.sin(a) * ry;
    points.push([
      cx + x * cr - y * sr,
      cy + x * sr + y * cr
    ]);
  }
  return points;
}

function ovalPoint(cx, cy, rx, ry, angle) {
  const a = angle * Math.PI / 180;
  return [cx + Math.cos(a) * rx, cy + Math.sin(a) * ry];
}

function mixColor(a, b, t) {
  return lerpColor(color(a), color(b), t);
}

function rgba(hex, alpha) {
  const c = color(hex);
  c.setAlpha(alpha);
  return c;
}
