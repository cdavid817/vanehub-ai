import { readFile, readdir, writeFile } from "node:fs/promises";
import { basename, join } from "node:path";

const [assetDirectory, tag, repository, outputPath] = process.argv.slice(2);
if (!assetDirectory || !tag || !repository || !outputPath) throw new Error("usage: <assets> <tag> <owner/repo> <output>");
const version = tag.replace(/^v/, "");
if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?$/.test(version)) throw new Error("invalid release version");

const entries = await readdir(assetDirectory, { recursive: true });
const signatures = entries.filter((entry) => entry.endsWith(".sig"));
const platforms = {};
for (const signaturePath of signatures) {
  const artifactPath = signaturePath.slice(0, -4);
  const name = basename(artifactPath).replaceAll(" ", ".");
  const lower = name.toLowerCase();
  const platform = lower.includes("macos-arm64") || lower.includes("aarch64") ? "darwin-aarch64"
    : lower.includes("macos-x64") ? "darwin-x86_64"
      : lower.includes("linux-x64") || lower.endsWith(".appimage") || lower.endsWith(".deb") ? "linux-x86_64"
        : lower.includes("windows-x64") || lower.endsWith(".exe") ? "windows-x86_64"
          : lower.endsWith(".dmg") ? "darwin-x86_64" : null;
  if (!platform || platforms[platform]) continue;
  const signature = (await readFile(join(assetDirectory, signaturePath), "utf8")).trim();
  if (signature.length < 40) throw new Error(`invalid signature for ${name}`);
  platforms[platform] = { signature, url: `https://github.com/${repository}/releases/download/${tag}/${encodeURIComponent(name)}` };
}
const requiredPlatforms = ["darwin-aarch64", "darwin-x86_64", "linux-x86_64", "windows-x86_64"];
const missingPlatforms = requiredPlatforms.filter((platform) => !platforms[platform]);
if (missingPlatforms.length > 0) throw new Error(`missing signed updater artifacts: ${missingPlatforms.join(", ")}`);
await writeFile(outputPath, `${JSON.stringify({ version, notes: `VaneHub AI ${tag}`, pub_date: new Date().toISOString(), platforms }, null, 2)}\n`);
