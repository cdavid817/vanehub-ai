import { execFileSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { measureSyntheticDatasets } from "./benchmark.mjs";
import { assertContentFree, compareManifest, formatFailures, loadJson, validateManifest } from "./harness.mjs";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(scriptDirectory, "../..");
const manifestPath = path.join(scriptDirectory, "fixtures/manifest.v1.json");
const command = process.argv[2] ?? "check";

try {
  const manifest = validateManifest(await loadJson(manifestPath), repositoryRoot);
  const provenance = { commit: sourceCommit(), platform: os.platform(), architecture: os.arch(), buildProfile: process.env.VANEHUB_PERFORMANCE_PROFILE ?? "test" };
  const results = compareManifest(manifest, provenance);
  assertContentFree(results);
  if (command === "benchmark") {
    const benchmarks = measureSyntheticDatasets(manifest);
    assertContentFree(benchmarks);
    process.stdout.write(`${JSON.stringify({ provenance, datasets: manifest.datasets.map(({ id, version }) => ({ id, version })), benchmarks }, null, 2)}\n`);
  } else if (command === "check") {
    const failures = formatFailures(results);
    process.stdout.write(`${JSON.stringify({ provenance, results, failures }, null, 2)}\n`);
    if (failures.length > 0) process.exitCode = 1;
  } else {
    throw new Error("performance-command-unknown");
  }
} catch (error) {
  process.stderr.write(`${error instanceof Error ? error.message : "performance-unknown-error"}\n`);
  process.exitCode = 1;
}

function sourceCommit() {
  try {
    return execFileSync("git", ["rev-parse", "HEAD"], { cwd: repositoryRoot, encoding: "utf8", windowsHide: true }).trim();
  } catch {
    return "unknown";
  }
}
