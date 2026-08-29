import { execFile } from "node:child_process";
import { chmod, copyFile, link, mkdir, mkdtemp, stat } from "node:fs/promises";
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

const SUPPORT_EXECUTABLES = process.platform === "win32"
  ? ["git.exe", "node.exe"]
  : ["git", "node"];

async function preserveSupportExecutable(source, target) {
  try {
    await stat(target);
    return;
  } catch {
    // The per-run support directory has not retained this executable yet.
  }
  try {
    await link(source, target);
  } catch {
    // A hard link can cross neither filesystems nor some Windows policy boundaries. Copying keeps
    // the fixture portable while still exposing only this allowlisted executable from the host.
    await copyFile(source, target);
  }
  if (process.platform !== "win32") await chmod(target, 0o755);
}

/**
 * `PATH` with the fixture directory first and every competing Agent installation removed.
 *
 * Prepending is not enough. Discovery enumerates the whole of `PATH`, so a developer's real
 * `claude.cmd` in `AppData\Roaming\npm` is found alongside the fixture, and two installations from
 * different sources with the fixture in front is a PATH-shadowing conflict -- which the launch
 * resolver refuses by design, reporting the Agent as unavailable while both copies sit there
 * working. The fixture has to be the only one, not merely the first.
 *
 * Only directories that actually hold a managed Agent launcher are dropped, so `node` and the
 * system directories the fixtures need stay reachable.
 */
export async function pathWithoutCompetingAgents(fixtureDir, inherited = process.env.PATH ?? "") {
  const windows = process.platform === "win32";
  const suffixes = windows ? [".cmd", ".exe", ".bat", ".ps1", ""] : [""];
  const kept = [];
  const supportDir = `${fixtureDir}-support`;
  let retainedSupport = false;
  for (const entry of inherited.split(path.delimiter)) {
    if (!entry) continue;
    const holdsAgent = await Promise.all(
      MANAGED_AGENT_EXECUTABLES.flatMap((name) => suffixes.map((suffix) =>
        stat(path.join(entry, `${name}${suffix}`)).then(() => true, () => false))),
    );
    if (!holdsAgent.includes(true)) {
      kept.push(entry);
      continue;
    }

    // A package-managed host may put Agent launchers beside foundational tools. Dropping the
    // whole directory removed `/usr/bin/git` and `/usr/bin/node` on Linux when Codex was installed
    // there, so native Git/MCP checks failed even though Agent isolation itself worked. Retain only
    // the explicit support allowlist in a run-scoped directory; no managed Agent can leak through.
    await mkdir(supportDir, { recursive: true });
    for (const executable of SUPPORT_EXECUTABLES) {
      const source = path.join(entry, executable);
      if (!await stat(source).then(() => true, () => false)) continue;
      await preserveSupportExecutable(source, path.join(supportDir, executable));
      retainedSupport = true;
    }
  }
  return [fixtureDir, ...(retainedSupport ? [supportDir] : []), ...kept].join(path.delimiter);
}

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
  // One directory per run, not per call. wdio evaluates this config in every worker, so a fresh
  // `mkdtemp` each time gave each spec file a different fixture directory -- and therefore a
  // different PATH and a different environment fingerprint -- inside a single layer that shares one
  // database. Deriving it from the run id keeps the layer's PATH identical for every spec in it.
  const runId = process.env.VANEHUB_TEST_RUN_ID;
  const directory = runId
    ? path.join(os.tmpdir(), `vanehub-agent-fixtures-${runId}`)
    : await mkdtemp(path.join(os.tmpdir(), "vanehub-agent-fixtures-"));
  await mkdir(directory, { recursive: true });
  for (const name of MANAGED_AGENT_EXECUTABLES) {
    const target = path.join(directory, windows ? `${name}.exe` : name);
    // Idempotent: a second worker preparing the same directory must not fail on an existing copy,
    // and must not rewrite a binary another worker's application may already be running.
    if (await cliFixtureIsCurrent(source, target)) continue;
    await copyFile(source, target);
    // The copy inherits the source's mode on POSIX, but only if the source is executable in the
    // checkout; setting it explicitly means a fresh clone cannot produce an Agent that will not run.
    if (!windows) await chmod(target, 0o755);
  }
  return directory;
}
