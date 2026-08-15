import { mkdir, mkdtemp, realpath, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { DesktopVerificationError } from "./verification-error.mjs";

const normalize = (value) => process.platform === "win32" ? value.toLowerCase() : value;
const isWithin = (parent, child) => {
  const relative = path.relative(parent, child);
  return relative !== "" && !relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative);
};

export function defaultApplicationDataDir(identifier = "ai.vanehub.app", env = process.env, platform = process.platform) {
  if (platform === "win32") return env.APPDATA ? path.join(env.APPDATA, identifier) : null;
  if (platform === "darwin") return env.HOME ? path.join(env.HOME, "Library", "Application Support", identifier) : null;
  const root = env.XDG_DATA_HOME ?? (env.HOME ? path.join(env.HOME, ".local", "share") : null);
  return root ? path.join(root, identifier) : null;
}

export function validateIsolatedDataPath({ runRoot, dataDir, normalDataDir }) {
  if (!path.isAbsolute(runRoot) || !path.isAbsolute(dataDir)) {
    throw new DesktopVerificationError("BLOCKED", "Desktop test paths must be absolute.");
  }
  const resolvedRoot = path.resolve(runRoot);
  const resolvedData = path.resolve(dataDir);
  if (!isWithin(resolvedRoot, resolvedData)) {
    throw new DesktopVerificationError("BLOCKED", "The desktop test data directory escapes its run root.");
  }
  if (normalDataDir && normalize(resolvedData) === normalize(path.resolve(normalDataDir))) {
    throw new DesktopVerificationError("BLOCKED", "The desktop test data directory aliases normal application data.");
  }
}

// Cleanup lives beside the code that created the run root so a passing run cannot leave the
// isolated SQLite database, configuration, or fixtures behind — CI would otherwise upload them
// as evidence from a job that had nothing to diagnose.
export async function disposeRunContext(context) {
  await rm(context.runRoot, { recursive: true, force: true });
  return { removed: context.runRoot, retainedEvidence: context.resultDir };
}

export async function createRunContext(repoRoot, options = {}) {
  const runId = options.runId ?? `${new Date().toISOString().replaceAll(/[:.]/g, "-")}-${randomUUID().slice(0, 8)}`;
  const runRoot = await mkdtemp(path.join(options.tempRoot ?? os.tmpdir(), "vanehub-desktop-e2e-"));
  const dataDir = path.join(runRoot, "data");
  const fixtureDir = path.join(runRoot, "fixtures");
  const resultDir = path.join(repoRoot, "test-results", "desktop", runId);
  await Promise.all([mkdir(dataDir), mkdir(fixtureDir), mkdir(resultDir, { recursive: true })]);
  const canonicalRoot = await realpath(runRoot);
  const canonicalData = await realpath(dataDir);
  validateIsolatedDataPath({
    runRoot: canonicalRoot,
    dataDir: canonicalData,
    normalDataDir: options.normalDataDir ?? defaultApplicationDataDir(),
  });
  return {
    runId,
    runRoot: canonicalRoot,
    dataDir: canonicalData,
    fixtureDir,
    resultDir,
    environment: {
      VANEHUB_APP_DATA_DIR: canonicalData,
      VANEHUB_TEST_RUN_ID: runId,
      VANEHUB_DESKTOP_RESULT_DIR: resultDir,
    },
  };
}
