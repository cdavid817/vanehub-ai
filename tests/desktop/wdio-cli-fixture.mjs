import { execFile } from "node:child_process";
import { stat } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

export const cliFixtureDir = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "fixtures",
  "cli",
);

const fixtureSource = path.join(cliFixtureDir, "opencode.rs");
const fixtureBinary = path.join(cliFixtureDir, "opencode.exe");

/** Whether the committed stub already matches its source, so rebuilding would change nothing. */
export async function cliFixtureIsCurrent(source = fixtureSource, binary = fixtureBinary) {
  const [sourceStat, binaryStat] = await Promise.all([
    stat(source).catch(() => null),
    stat(binary).catch(() => null),
  ]);
  if (!sourceStat || !binaryStat) return false;
  return binaryStat.mtimeMs >= sourceStat.mtimeMs;
}

/**
 * Compiles the fake `opencode` the CLI-terminal, session-workspace and dialogs layers drive.
 *
 * Skipped when the committed binary is already newer than its source, which is the normal case --
 * the stub is checked in. Rebuilding unconditionally is what broke the full suite on Windows: three
 * layers call this, an earlier layer has just run the stub, and Windows had not released its handle
 * yet, so `rustc` could not write the file. That surfaces as `LNK1104: cannot open file` at config
 * load, which fails the entire layer before a single spec runs, and it only happens when the layers
 * run back to back -- so the targeted layers stayed green and the full suite did not.
 */
export async function prepareCliFixture() {
  if (process.platform !== "win32") return;
  if (await cliFixtureIsCurrent()) return;
  await promisify(execFile)("rustc", [fixtureSource, "-o", fixtureBinary]);
}
