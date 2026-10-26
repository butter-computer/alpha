// Build the contact sheet: node contact.mjs <outDir>  (reads alpha-icon-v*.png there, writes alpha-icon-contact-sheet.png)
import http from "node:http";
import { readFileSync, writeFileSync, readdirSync, existsSync } from "node:fs";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import path from "node:path";
const run = promisify(execFile);
const outDir = path.resolve(process.argv[2] || "out");
const files = readdirSync(outDir).filter(f => /^alpha-icon-v\d-.*\.png$/.test(f)).sort();
const cell = f => {
  const label = f.replace(/^alpha-icon-/, "").replace(/\.png$/, "");
  return `<figure><img class="big" src="${f}"><div class="small"><img width="128" src="${f}"><img width="32" src="${f}"><span>128 · 32</span></div><figcaption>${label}</figcaption></figure>`;
};
const html = `<!doctype html><meta charset="utf-8"><style>
body{margin:0;padding:36px;background:#ece8e1;font:13px/1.3 -apple-system,Helvetica,sans-serif;color:#555;width:1608px}
.grid{display:grid;grid-template-columns:repeat(3,1fr);gap:36px 30px}
figure{margin:0;background:#f7f5f1;border-radius:14px;padding:18px;box-shadow:0 1px 0 rgba(0,0,0,.06)}
.big{width:100%;display:block}.small{display:flex;align-items:center;gap:22px;margin:4px 8px 0}
.small span{color:#999;font-size:11px;letter-spacing:.04em}figcaption{margin:8px 8px 0;font-weight:600;color:#333}
</style><div class="grid">${files.map(cell).join("")}</div>`;
writeFileSync(path.join(outDir, "contact.html"), html);
const server = http.createServer((req, res) => {
  const f = path.join(outDir, path.basename(new URL(req.url, "http://x").pathname));
  if (!existsSync(f)) { res.statusCode = 404; return res.end(); }
  res.setHeader("content-type", f.endsWith(".png") ? "image/png" : "text/html");
  res.end(readFileSync(f));
});
await new Promise(r => server.listen(0, "127.0.0.1", r));
const ab = (...a) => run("agent-browser", ["--session", "alpha-icon-gen", ...a], { timeout: 120000 });
try {
  await ab("open", `http://127.0.0.1:${server.address().port}/contact.html`);
  await ab("set", "viewport", "1680", "1500");
  await ab("wait", "--load", "networkidle");
  await ab("screenshot", "--full", path.join(outDir, "alpha-icon-contact-sheet.png"));
} finally { try { await ab("close"); } catch {} server.close(); }
