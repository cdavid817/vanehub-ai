import { readFile } from "node:fs/promises";
import { stdout } from "node:process";
import { URL } from "node:url";

const manifestUrl = new URL("../dist/.vite/manifest.json", import.meta.url);
const manifest = JSON.parse(await readFile(manifestUrl, "utf8"));
const requiredDynamicEntries = [
  "src/loop-center/loop-center.tsx",
  "src/session-workspace/logs-tab.tsx",
  "src/settings/pages/agents-page.tsx",
  "src/settings/pages/prompt-hooks-page.tsx",
];

for (const source of requiredDynamicEntries) {
  const entry = Object.values(manifest).find((candidate) => candidate.src === source);
  if (!entry?.isDynamicEntry) {
    throw new Error(`Expected ${source} to be emitted as a dynamic entry.`);
  }
}

stdout.write(`Verified ${requiredDynamicEntries.length} lazy frontend chunks.\n`);
