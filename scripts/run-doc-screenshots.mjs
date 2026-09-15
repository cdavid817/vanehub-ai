import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { repositoryRoot, run } from "./docs-tooling.mjs";
import { allocateScreenshotPort } from "./docs-screenshot-port.mjs";

const mode = process.argv[2];
if (!["update", "check"].includes(mode)) {
  console.error("Usage: node scripts/run-doc-screenshots.mjs <update|check>");
  process.exit(2);
}

const port = await allocateScreenshotPort(process.env.DOCS_SCREENSHOT_PORT);

run(process.execPath, [
  resolve(repositoryRoot, "node_modules", "@playwright", "test", "cli.js"),
  "test",
  "--config",
  resolve(repositoryRoot, "playwright.docs.config.ts"),
], {
  env: {
    ...process.env,
    DOCS_SCREENSHOT_PORT: String(port),
    DOCS_SCREENSHOT_MODE: mode,
  },
});

// The inventory records which source revision the committed images came from. Only an update
// run rewrites it: `check` compares pixels and must not touch the manifest.
if (mode === "update") {
  const inventoryPath = resolve(repositoryRoot, "docs", "user-guide", "screenshots.json");
  const inventory = JSON.parse(readFileSync(inventoryPath, "utf8"));
  inventory.capture = {
    ...inventory.capture,
    sourceCommit: execFileSync("git", ["rev-parse", "HEAD"], { cwd: repositoryRoot, encoding: "utf8" }).trim(),
    generatedAt: new Date().toISOString(),
  };
  writeFileSync(inventoryPath, `${JSON.stringify(inventory, null, 2)}\n`, "utf8");
}
