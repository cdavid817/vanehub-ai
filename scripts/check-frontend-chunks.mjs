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
  "src/settings/pages/agent-configurations-page.tsx",
  "src/settings/pages/basic-settings-page.tsx",
  "src/settings/cli-parameters/cli-parameters-page.tsx",
  "src/settings/pages/cli-management/cli-management-page.tsx",
  "src/settings/pages/extensions-page.tsx",
  "src/settings/pages/im-page.tsx",
  "src/settings/pages/mcp-page.tsx",
  "src/settings/pages/observability-settings-page.tsx",
  "src/settings/pages/plugin-integrations-page.tsx",
  "src/settings/pages/prompt-hooks-page.tsx",
  "src/settings/pages/skills-page.tsx",
  "src/settings/pages/ssh-connections-page.tsx",
  "src/settings/pages/usage-statistics-page.tsx",
];
const maxStaticEntryGzipBytes = 350 * 1024;
// Raised from 700 KiB by `fix-session-creation-and-trace-correctness`, the first raise since this
// ceiling was set. `main` builds the App chunk at 714,287 bytes; this branch takes it to 716,933.
// The +2,646 is the session-creation dialog: the local path no longer runs through a hidden SSH
// check, directory checks are ordered instead of racing each other, a failed operation completion
// is reported rather than discarded, and the remembered Agent is applied once its list arrives.
// All of it is behaviour that did not exist, none of it a copy of something else -- the dialog's
// own file shrank as those four moved into named hooks.
//
// Kept as a round figure with headroom rather than pinned to a measurement, unlike the line
// budgets. There are two builds behind this one number: `npm run build` emits 716,933 bytes and
// the `VITE_DESKTOP_E2E=1` build behind `scripts/build-desktop-e2e.mjs` emits 717,234. A ceiling
// recorded at either measurement fails the other, which is how this was found -- the desktop
// smoke job runs the second build and nothing local runs it by default. Measure that one when
// raising this, since it is always the larger of the two.
//
// Raised from 701 KiB by `harden-session-workspace-tab-layouts`, measured after merging the
// session-creation raise above: `npm run build` emits 718,691 bytes and the desktop-e2e build
// 719,005. The +1,758 over the previous `main` is the full-width files toolbar with its selected
// path, the session-row and badge width constraints, the collapsible CLI composer strip, and
// the CLI theme attribute plumbing in the settings provider -- all of it in files the App chunk
// already owned, none of it a second copy of a lazy panel.
//
// Raised from 704 KiB by `extend-cli-providers-with-acp`, measured after merging main: `npm run
// build` emits 721,101 bytes (704.2 KiB). The growth over main is the seven expanded CLIs in the
// static closure -- their registry entries, the create-session legacy group, the CLI management
// connection actions and status badges, the Agent configuration selector's two new targets, and
// the brand-mark imports -- all in files the App chunk already owned. 705 keeps the same
// headroom the previous raises left for the slightly larger desktop-e2e build.
const maxRawJavaScriptChunkBytes = 705 * 1024;

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
