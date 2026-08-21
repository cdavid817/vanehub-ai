import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { createDesktopConfig } from "./wdio-shared.mjs";

const fixtureDir = path.join(path.dirname(fileURLToPath(import.meta.url)), "fixtures", "cli");

// Ahead of the inherited PATH so the native runtime resolves the builtin `opencode` Agent to the
// fixture even on a host where the real one is installed. Every other layer keeps the plain PATH.
export const config = await createDesktopConfig({
  specDirectory: "specs-cli-terminal",
  environment: { PATH: `${fixtureDir}${path.delimiter}${process.env.PATH ?? ""}` },
});
