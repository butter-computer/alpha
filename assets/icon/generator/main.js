// p5 + p5.brush entry: ?v=v1..v6 selects the variant, ?seed=N pins the PRNG, ?preview=1 skips the upload.
const Q = new URLSearchParams(location.search);
const VARIANT = Q.get("v") || "v1";

async function setup() {
  createCanvas(1200, 1200, WEBGL);
  brush.scaleBrushes(6);
  angleMode(DEGREES);
  randomSeed(41);
  noiseSeed(41);
  background("#30241e");
}

function draw() {
  translate(-width / 2, -height / 2);
  brush.noField();
  VARIANTS[VARIANT].fn();
  noLoop();
  setTimeout(finish, 300);
}

// macOS icon grid: 824px squircle tile centred in 1024px, ~10% transparent margin, soft drop shadow.
function squirclePath(ctx, cx, cy, half, n = 5) {
  ctx.beginPath();
  for (let i = 0; i <= 360; i++) {
    const t = i * Math.PI / 180, c = Math.cos(t), s = Math.sin(t);
    const x = cx + half * Math.sign(c) * Math.pow(Math.abs(c), 2 / n);
    const y = cy + half * Math.sign(s) * Math.pow(Math.abs(s), 2 / n);
    i ? ctx.lineTo(x, y) : ctx.moveTo(x, y);
  }
  ctx.closePath();
}

async function finish() {
  const src = document.querySelector("canvas");
  const out = document.createElement("canvas");
  out.width = out.height = 1024;
  const ctx = out.getContext("2d");
  ctx.imageSmoothingQuality = "high";
  ctx.save();
  ctx.shadowColor = "rgba(0,0,0,0.30)";
  ctx.shadowBlur = 24;
  ctx.shadowOffsetY = 10;
  squirclePath(ctx, 512, 512, 412);
  ctx.fillStyle = "#000";
  ctx.fill();
  ctx.restore();
  ctx.save();
  squirclePath(ctx, 512, 512, 412);
  ctx.clip();
  ctx.drawImage(src, 100, 100, 824, 824);
  ctx.restore();
  squirclePath(ctx, 512, 512, 411);
  ctx.lineWidth = 2;
  ctx.strokeStyle = "rgba(0,0,0,0.22)";
  ctx.stroke();
  const blob = await new Promise(r => out.toBlob(r, "image/png"));
  if (!Q.get("preview")) {
    await fetch("/save?name=" + encodeURIComponent(VARIANT), { method: "POST", body: blob });
  } else {
    document.body.appendChild(out); out.style.cssText = "position:fixed;left:0;top:0;width:512px;z-index:9;background:#888";
  }
  window.__done = true;
}
