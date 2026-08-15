// Copies release bundles (.app / .dmg) from Tauri's deep target dir into a
// single, predictable `dist/` folder at the project root.
// Run after `tauri build` (see package.json "dist" script).
import { cpSync, mkdirSync, readdirSync, existsSync, statSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const bundleDir = join(root, "src-tauri", "target", "release", "bundle");
const distDir = join(root, "dist");

if (!existsSync(bundleDir)) {
  console.error("未找到 bundle 目录，请先运行: npm run build");
  process.exit(1);
}

mkdirSync(distDir, { recursive: true });

const copied = [];
for (const sub of ["macos", "dmg", "deb", "rpm", "nsis", "msi", "appimage"]) {
  const dir = join(bundleDir, sub);
  if (!existsSync(dir)) continue;
  for (const name of readdirSync(dir)) {
    const src = join(dir, name);
    if (!statSync(src).isFile() && !name.endsWith(".app")) continue;
    const dest = join(distDir, name);
    cpSync(src, dest, { recursive: true, force: true });
    copied.push(dest);
  }
}

console.log(`\n产物已复制到 ${distDir}/`);
for (const f of copied) console.log(`  - ${f}`);
