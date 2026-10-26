// Renders the cuelume (https://cuelume.dev, MIT) cues src/sound.rs plays into
// assets/sounds, through the library's own renderRecipe/createOutput so each
// file matches what play() makes in a browser. Cues cuelume varies per play
// get several takes.
//
//   npm install --prefix /tmp/cuelume cuelume node-web-audio-api
//   node scripts/render-sounds.mjs /tmp/cuelume/node_modules assets/sounds
import { createRequire } from "node:module";
import { writeFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
const [mods, out] = process.argv.slice(2);
const pkg = join(mods, "cuelume");
const { OfflineAudioContext } = createRequire(join(mods, "x.js"))("node-web-audio-api");
const dist = join(pkg, "dist");
const { RECIPES } = await import(join(dist, "sounds/recipes.js"));
const { shapeFor, contextFrom, resolveEmphasis } = await import(join(dist, "sounds/context.js"));
const { createOutput, ROOM_SECONDS } = await import(join(dist, "audio/output.js"));
const { renderRecipe, layersFor } = await import(join(dist, "audio/render.js"));

// [file, cue, emphasis, options, variants]
const CUES = [
  ["heading", "select", "strong", {}, 1],
  ["block", "select", "normal", {}, 1],
  ["item", "select", "subtle", {}, 1],
  ["inline", "tap", "subtle", { input: "keyboard" }, 4],
  ["code", "open", "normal", {}, 1],
  ["check", "toggle", "normal", { direction: "forward" }, 1],
  ["uncheck", "toggle", "normal", { direction: "back" }, 1],
  ["saved", "success", "subtle", {}, 1],
  ["exported", "ready", "normal", {}, 1],
];
const RATE = 48000;
mkdirSync(out, { recursive: true });

function wav(left, right) {
  const n = left.length, buf = Buffer.alloc(44 + n * 4);
  buf.write("RIFF", 0); buf.writeUInt32LE(36 + n * 4, 4); buf.write("WAVE", 8);
  buf.write("fmt ", 12); buf.writeUInt32LE(16, 16); buf.writeUInt16LE(1, 20); buf.writeUInt16LE(2, 22);
  buf.writeUInt32LE(RATE, 24); buf.writeUInt32LE(RATE * 4, 28); buf.writeUInt16LE(4, 32); buf.writeUInt16LE(16, 34);
  buf.write("data", 36); buf.writeUInt32LE(n * 4, 40);
  const s = (v) => Math.max(-32768, Math.min(32767, Math.round(v * 32767)));
  for (let i = 0; i < n; i++) { buf.writeInt16LE(s(left[i]), 44 + i * 4); buf.writeInt16LE(s(right[i]), 46 + i * 4); }
  return buf;
}

for (const [file, sound, em, options, variants] of CUES) {
  for (let v = 0; v < variants; v++) {
    const recipe = RECIPES[sound];
    const emphasis = resolveEmphasis(em);
    const played = { recipe, volume: 1, emphasis, shape: shapeFor(sound, contextFrom(options), emphasis, Infinity), step: 0 };
    const end = Math.max(...layersFor(played, "both").map((l) => (l.offset ?? 0) + l.attack + l.decay));
    const ctx = new OfflineAudioContext(2, Math.ceil((end + ROOM_SECONDS + 0.1) * RATE), RATE);
    let bus = null;
    renderRecipe(ctx, () => (bus ??= createOutput(ctx)), new Map(), sound, played, "both");
    const buffer = await ctx.startRendering();
    const [l, r] = [buffer.getChannelData(0), buffer.getChannelData(1)];
    let last = l.length - 1;
    while (last > 0 && Math.abs(l[last]) < 1e-4 && Math.abs(r[last]) < 1e-4) last--;
    const n = Math.min(l.length, last + 1 + Math.round(0.005 * RATE));
    // 5 ms fade so the trim never clicks
    const fade = Math.round(0.005 * RATE);
    for (let i = 0; i < fade && n - 1 - i >= 0; i++) { const g = i / fade; l[n - 1 - i] *= g; r[n - 1 - i] *= g; }
    const name = variants > 1 ? `${file}-${v + 1}.wav` : `${file}.wav`;
    let peak = 0; for (let i = 0; i < n; i++) peak = Math.max(peak, Math.abs(l[i]), Math.abs(r[i]));
    writeFileSync(join(out, name), wav(l.subarray(0, n), r.subarray(0, n)));
    console.log(name.padEnd(14), `${(n / RATE * 1000).toFixed(0)}ms`, `peak ${peak.toFixed(3)}`);
  }
}
process.exit(0);
