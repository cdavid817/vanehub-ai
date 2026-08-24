import { execFile } from "node:child_process";
import { chmod, copyFile, mkdtemp, stat } from "node:fs/promises";
import os from "node:os";
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

/**
 * The executable name of every CLI Agent the native registry manages.
 *
 * Kept in the same order as `registry.rs`. A managed Agent missing from this list is an Agent the
 * required gate cannot stand up, which is how `desktop-smoke` came to need a real `codex` on PATH.
 */
export const MANAGED_AGENT_EXECUTABLES = ["claude", "codex", "gemini", "opencode", "agy"];

/**
 * Materialises the fixture stub under every managed Agent name in a fresh temporary directory.
 *
 * Copies rather than recompiles: one stub answers `--version` and echoes stdin for all of them, and
 * the behaviour under test is this application's -- resolution, launch, session creation,
 * persistence -- not any vendor's. It is the same stub and the same protocol the CLI-terminal layer
 * already drives, so there is no second fixture framework here.
 *
 * A temporary directory rather than the repository: these are generated, and a run that writes five
 * executables into a tracked directory leaves the working tree dirty on every machine that runs it.
 */
export async function prepareManagedCliFixtures() {
  await prepareCliFixture();
  const windows = process.platform === "win32";
  const source = windows ? fixtureBinary : path.join(cliFixtureDir, "opencode");
  const directory = await mkdtemp(path.join(os.tmpdir(), "vanehub-agent-fixtures-"));
  for (const name of MANAGED_AGENT_EXECUTABLES) {
    const target = path.join(directory, windows ? `${name}.exe` : name);
    await copyFile(source, target);
    // The copy inherits the source's mode on POSIX, but only if the source is executable in the
    // checkout; setting it explicitly means a fresh clone cannot produce an Agent that will not run.
    if (!windows) await chmod(target, 0o755);
  }
  return directory;
}
