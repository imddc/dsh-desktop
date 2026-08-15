// Prepares the bundled runtime for the .app:
//   1. downloads the official Node.js darwin-arm64 binary
//   2. installs @deepseek-ai/dsh into src-tauri/resources/dsh
// Run once before `tauri build`; CI runs it automatically.
// Override with NODE_VERSION / DSH_VERSION env vars.
import { execSync } from "node:child_process";
import { existsSync, mkdirSync, rmSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const resDir = join(root, "src-tauri", "resources");
const nodeDir = join(resDir, "node");
const dshDir = join(resDir, "dsh");

const NODE_VERSION = process.env.NODE_VERSION || "v24.19.0";
const DSH_PACKAGE = "@deepseek-ai/dsh";
const DSH_VERSION = process.env.DSH_VERSION || "latest"; // npm semver range
const ARCH = process.env.ARCH || "arm64"; // darwin arm64 (Apple Silicon)

function sh(cmd, opts = {}) {
  console.log(`\n$ ${cmd}`);
  execSync(cmd, { stdio: "inherit", ...opts });
}

// ---------- 1. Node.js binary ----------
if (existsSync(join(nodeDir, "bin", "node"))) {
  console.log(`✓ node 已存在: ${join(nodeDir, "bin", "node")}`);
} else {
  console.log(`\n▶ 下载 Node.js ${NODE_VERSION} (darwin-${ARCH}) ...`);
  mkdirSync(nodeDir, { recursive: true });
  const tarball = `node-v${NODE_VERSION.replace(/^v/, "")}-darwin-${ARCH}.tar.gz`;
  const url = `https://nodejs.org/dist/${NODE_VERSION}/${tarball}`;
  sh(`curl -fL --retry 3 -o /tmp/${tarball} "${url}"`);
  sh(`tar -xzf /tmp/${tarball} -C /tmp`);
  const srcBin = `/tmp/node-v${NODE_VERSION.replace(/^v/, "")}-darwin-${ARCH}/bin/node`;
  sh(`mkdir -p "${nodeDir}/bin" && cp "${srcBin}" "${nodeDir}/bin/node" && chmod +x "${nodeDir}/bin/node"`);
  sh(`rm -rf /tmp/${tarball} /tmp/node-v${NODE_VERSION.replace(/^v/, "")}-darwin-${ARCH}`);
  console.log(`✓ node 就绪: ${join(nodeDir, "bin", "node")}`);
}

// ---------- 2. dsh package ----------
if (existsSync(join(dshDir, "node_modules", "@deepseek-ai", "dsh"))) {
  console.log(`✓ dsh 已存在: ${join(dshDir, "node_modules", "@deepseek-ai", "dsh")}`);
} else {
  console.log(`\n▶ 安装 ${DSH_PACKAGE}@${DSH_VERSION} ...`);
  mkdirSync(dshDir, { recursive: true });
  // Install into the runtime dir with a scoped package.json so the dependency
  // tree is self-contained (key = package name, value = version).
  const fs = await import("node:fs");
  const pkg = {
    name: "dsh-runtime",
    private: true,
    dependencies: { [DSH_PACKAGE]: DSH_VERSION },
  };
  fs.writeFileSync(join(dshDir, "package.json"), JSON.stringify(pkg, null, 2));
  sh(`npm install --prefix "${dshDir}" --no-audit --no-fund --omit=dev`);
  console.log(`✓ dsh 就绪: ${join(dshDir, "node_modules", "@deepseek-ai", "dsh")}`);
}

// ---------- report ----------
const { statSync } = await import("node:fs");
function size(p) {
  if (!existsSync(p)) return "N/A";
  const s = statSync(p);
  return s.isDirectory() ? "dir" : `${(s.size / 1024 / 1024).toFixed(1)}MB`;
}
console.log(`\n=== 捆绑运行时清单 ===`);
console.log(`  node : ${join(nodeDir, "bin", "node")}  (${size(join(nodeDir, "bin", "node"))})`);
console.log(`  dsh  : ${join(dshDir, "node_modules", "@deepseek-ai", "dsh")}`);
