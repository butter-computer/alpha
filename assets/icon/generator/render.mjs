// Re-run the generator: node render.mjs <outDir> [v1 v2 ...]
// Serves this directory, loads each variant in headless Chrome via the agent-browser CLI,
// receives the composed 1024px PNG from the page and writes <outDir>/alpha-icon-<v>-<name>.png.
// Needs network access for the pinned p5 / p5.brush / polygon-clipping CDN scripts.
import http from "node:http";
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
const run = promisify(execFile); // async: a sync exec would block the server the page loads from
import path from "node:path";

const here = path.dirname(new URL(import.meta.url).pathname);
const outDir = path.resolve(process.argv[2] || "out");
const names = { v1: "parquet-inlay", v2: "rosewood-oval", v3: "pieced-grain", v4: "open-book", v5: "minimal-seam", v6: "sunburst-rosette", play: "playground" }; // play = interactive.html defaults (Playfair Display 700)
const which = process.argv.slice(3).length ? process.argv.slice(3) : Object.keys(names).filter(v => v !== "play");
mkdirSync(outDir, { recursive: true });

const mime = { ".html": "text/html", ".js": "text/javascript" };
let pending = null;
const server = http.createServer((req, res) => {
  const u = new URL(req.url, "http://x");
  if (req.method === "POST" && u.pathname === "/save") {
    const chunks = [];
    req.on("data", c => chunks.push(c));
    req.on("end", () => {
      const v = u.searchParams.get("name");
      const file = path.join(outDir, `alpha-icon-${v}-${names[v]}.png`);
      writeFileSync(file, Buffer.concat(chunks));
      console.log("wrote", file);
      res.end("ok");
    });
    return;
  }
  const f = path.join(here, u.pathname === "/" ? "index.html" : path.basename(u.pathname));
  if (!existsSync(f)) { res.statusCode = 404; return res.end(); }
  res.setHeader("content-type", mime[path.extname(f)] || "application/octet-stream");
  res.end(readFileSync(f));
});
await new Promise(r => server.listen(0, "127.0.0.1", r));
const port = server.address().port;
const SESSION = "alpha-icon-gen"; // dedicated session so other agent-browser users are untouched
const ab = (...a) => run("agent-browser", ["--session", SESSION, ...a], { timeout: 300000 });
try {
  for (const v of which) {
    await ab("open", v === "play" ? `http://127.0.0.1:${port}/interactive.html?auto=1` : `http://127.0.0.1:${port}/?v=${v}`);
    await ab("wait", "--fn", "window.__done === true");
  }
} finally {
  try { await ab("close"); } catch {}
  server.close();
}
