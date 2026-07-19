import { spawnSync } from "node:child_process";
import { Buffer } from "node:buffer";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, "..");
const outputDir = resolve(process.argv[2] ?? join(repoRoot, "src-tauri", "icons"));
const sourceDir = join(outputDir, "source");
const generatedDir = join(outputDir, "generated");
const opticalDir = join(outputDir, "optical");
const rasterDir = join(outputDir, "raster");
const publicDir = join(repoRoot, "public");

const manifestPath = join(sourceDir, "icon-manifest.json");
const masterPath = join(sourceDir, "app-icon.svg");
const compactPath = join(sourceDir, "app-icon-compact.svg");

for (const requiredPath of [manifestPath, masterPath, compactPath]) {
  if (!existsSync(requiredPath)) {
    throw new Error(`Missing icon source: ${requiredPath}`);
  }
}

for (const directory of [generatedDir, opticalDir, rasterDir, publicDir]) {
  mkdirSync(directory, { recursive: true });
}

function runTauriIcon(args) {
  const tauriCli = join(repoRoot, "node_modules", "@tauri-apps", "cli", "tauri.js");
  if (!existsSync(tauriCli)) {
    throw new Error("Missing @tauri-apps/cli. Run npm install before generating icons.");
  }

  const result = spawnSync(process.execPath, [tauriCli, "icon", ...args], {
    cwd: repoRoot,
    stdio: "inherit",
  });

  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`Tauri icon generation failed with exit code ${result.status}`);
  }
}

runTauriIcon([manifestPath, "-o", generatedDir]);
runTauriIcon([compactPath, "-o", opticalDir, "-p", "16", "-p", "24", "-p", "32"]);
runTauriIcon([
  masterPath,
  "-o",
  rasterDir,
  "-p",
  "48",
  "-p",
  "64",
  "-p",
  "128",
  "-p",
  "180",
  "-p",
  "192",
  "-p",
  "256",
  "-p",
  "512",
]);

copyFileSync(join(opticalDir, "32x32.png"), join(generatedDir, "32x32.png"));

const icoSources = [
  [16, join(opticalDir, "16x16.png")],
  [24, join(opticalDir, "24x24.png")],
  [32, join(opticalDir, "32x32.png")],
  [48, join(rasterDir, "48x48.png")],
  [64, join(rasterDir, "64x64.png")],
  [128, join(rasterDir, "128x128.png")],
  [256, join(rasterDir, "256x256.png")],
].map(([size, path]) => ({ size, bytes: readFileSync(path) }));

const directorySize = 6 + icoSources.length * 16;
const payloadSize = icoSources.reduce((sum, entry) => sum + entry.bytes.length, 0);
const ico = Buffer.alloc(directorySize + payloadSize);
ico.writeUInt16LE(0, 0);
ico.writeUInt16LE(1, 2);
ico.writeUInt16LE(icoSources.length, 4);

let payloadOffset = directorySize;
for (const [index, entry] of icoSources.entries()) {
  const offset = 6 + index * 16;
  const encodedSize = entry.size === 256 ? 0 : entry.size;
  ico.writeUInt8(encodedSize, offset);
  ico.writeUInt8(encodedSize, offset + 1);
  ico.writeUInt8(0, offset + 2);
  ico.writeUInt8(0, offset + 3);
  ico.writeUInt16LE(1, offset + 4);
  ico.writeUInt16LE(32, offset + 6);
  ico.writeUInt32LE(entry.bytes.length, offset + 8);
  ico.writeUInt32LE(payloadOffset, offset + 12);
  entry.bytes.copy(ico, payloadOffset);
  payloadOffset += entry.bytes.length;
}
writeFileSync(join(generatedDir, "icon.ico"), ico);

copyFileSync(compactPath, join(publicDir, "favicon.svg"));
copyFileSync(join(opticalDir, "32x32.png"), join(publicDir, "favicon-32.png"));
copyFileSync(join(rasterDir, "180x180.png"), join(publicDir, "apple-touch-icon.png"));
copyFileSync(join(rasterDir, "192x192.png"), join(publicDir, "icon-192.png"));
copyFileSync(join(rasterDir, "512x512.png"), join(publicDir, "icon-512.png"));

function listFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? listFiles(path) : [{ path, bytes: statSync(path).size }];
  });
}

for (const file of listFiles(outputDir).sort((left, right) => left.path.localeCompare(right.path))) {
  process.stdout.write(`${file.path}\t${file.bytes}\n`);
}
