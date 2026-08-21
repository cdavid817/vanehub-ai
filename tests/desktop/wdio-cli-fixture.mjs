import { execFile } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

export const cliFixtureDir = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "fixtures",
  "cli",
);

export async function prepareCliFixture() {
  if (process.platform !== "win32") return;
  await promisify(execFile)("rustc", [
    path.join(cliFixtureDir, "opencode.rs"),
    "-o",
    path.join(cliFixtureDir, "opencode.exe"),
  ]);
}
