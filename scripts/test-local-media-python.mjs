#!/usr/bin/env node
/**
 * Runs the local-media Python bridge tests.
 *
 * The bridge is the one part of this product written in Python, and it has no build step that
 * would notice a syntax error, so it needs its own gate. The suite uses only the standard library:
 * requiring pytest would make the gate depend on a package this repository does not otherwise
 * install, and CI would skip it the first time that install failed.
 *
 * Exits 0 with a stated reason when no interpreter is present. That is a truthful "NOT RUN", not a
 * pass -- the packaged worker resolves whichever interpreter the user configured, and a build
 * machine without Python is not evidence of a defect.
 */

import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const workerRoot = join(repositoryRoot, "src-tauri", "resources", "local-media-worker");

if (!existsSync(join(workerRoot, "tests"))) {
  console.error(`local-media worker tests are missing at ${workerRoot}`);
  process.exit(1);
}

const candidates = process.env.VANEHUB_PYTHON
  ? [process.env.VANEHUB_PYTHON]
  : process.platform === "win32"
    ? ["python", "py", "python3"]
    : ["python3", "python"];

function probe(executable) {
  const probed = spawnSync(executable, ["--version"], { encoding: "utf8" });
  return probed.status === 0 ? (probed.stdout || probed.stderr).trim() : null;
}

const interpreter = candidates.map((name) => [name, probe(name)]).find(([, version]) => version);

if (!interpreter) {
  console.log(
    `NOT RUN: no Python interpreter found (tried ${candidates.join(", ")}). ` +
      "Set VANEHUB_PYTHON to run the local-media bridge tests.",
  );
  process.exit(0);
}

const [executable, version] = interpreter;
console.log(`Running local-media bridge tests with ${executable} (${version})`);

const result = spawnSync(
  executable,
  // `-W error::ResourceWarning` would be nice here but the standard library itself trips it on
  // some platforms; the suite closes what it opens and is checked by review instead.
  ["-m", "unittest", "discover", "-s", "tests", "-t", "."],
  { cwd: workerRoot, stdio: "inherit" },
);

if (result.error) {
  console.error(`failed to run ${executable}: ${result.error.message}`);
  process.exit(1);
}
process.exit(result.status ?? 1);
