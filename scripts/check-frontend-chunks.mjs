import { readFile } from "node:fs/promises";
import { stdout } from "node:process";
import { URL } from "node:url";
import { gzipSync } from "node:zlib";

const manifestUrl = new URL("../dist/.vite/manifest.json", import.meta.url);
const distUrl = new URL("../dist/", import.meta.url);
const manifest = JSON.parse(await readFile(manifestUrl, "utf8"));
const requiredDynamicEntries = [
  "src/loop-center/loop-center.tsx",
  "src/session-workspace/logs-tab.tsx",
  "src/settings/pages/about-page.tsx",
  "src/settings/pages/basic-settings-page.tsx",
  "src/settings/pages/cli-parameters-page.tsx",
  "src/settings/pages/extensions-page.tsx",
  "src/settings/pages/im-page.tsx",
  "src/settings/pages/mcp-page.tsx",
  "src/settings/pages/observability-settings-page.tsx",
  "src/settings/pages/plugin-integrations-page.tsx",
  "src/settings/pages/providers-page.tsx",
  "src/settings/pages/skills-page.tsx",
  "src/settings/pages/ssh-connections-page.tsx",
  "src/settings/pages/usage-statistics-page.tsx",
  "src/settings/pages/agents-page.tsx",
  "src/settings/pages/prompt-hooks-page.tsx",
];
const maxStaticEntryGzipBytes = 350 * 1024;
const maxRawJavaScriptChunkBytes = 700 * 1024;

for (const source of requiredDynamicEntries) {
  const entry = Object.values(manifest).find((candidate) => candidate.src === source);
  if (!entry?.isDynamicEntry) {
    throw new Error(`Expected ${source} to be emitted as a dynamic entry.`);
  }
}

const fileBytes = new Map();
for (const entry of Object.values(manifest)) {
  if (!entry.file.endsWith(".js") || fileBytes.has(entry.file)) continue;
  const content = await readFile(new URL(entry.file, distUrl));
  fileBytes.set(entry.file, content);
  if (content.byteLength > maxRawJavaScriptChunkBytes) {
    throw new Error(
      `${entry.file} is ${formatKiB(content.byteLength)} raw; budget is ${formatKiB(maxRawJavaScriptChunkBytes)}.`,
    );
  }
}

const mainEntry = manifest["index.html"];
if (!mainEntry?.isEntry) throw new Error("Expected index.html to be the main Vite entry.");
const staticEntryKeys = collectStaticImports("index.html");
const staticEntryGzipBytes = [...staticEntryKeys].reduce((total, key) => {
  const entry = manifest[key];
  const content = entry?.file ? fileBytes.get(entry.file) : undefined;
  return total + (content ? gzipSync(content).byteLength : 0);
}, 0);
if (staticEntryGzipBytes > maxStaticEntryGzipBytes) {
  throw new Error(
    `Main static JavaScript closure is ${formatKiB(staticEntryGzipBytes)} gzip; budget is ${formatKiB(maxStaticEntryGzipBytes)}.`,
  );
}

stdout.write(
  `Verified ${requiredDynamicEntries.length} lazy frontend chunks; main static closure ${formatKiB(staticEntryGzipBytes)} gzip.\n`,
);

function collectStaticImports(rootKey) {
  const visited = new Set();
  const pending = [rootKey];
  while (pending.length > 0) {
    const key = pending.pop();
    if (!key || visited.has(key)) continue;
    visited.add(key);
    for (const importedKey of manifest[key]?.imports ?? []) pending.push(importedKey);
  }
  return visited;
}

function formatKiB(bytes) {
  return `${(bytes / 1024).toFixed(1)} KiB`;
}
