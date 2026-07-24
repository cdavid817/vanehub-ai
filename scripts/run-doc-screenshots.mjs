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
