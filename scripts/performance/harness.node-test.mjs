import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  applyOverrides,
  assertContentFree,
  compareManifest,
  formatFailures,
  loadJson,
  validateManifest,
} from "./harness.mjs";

const directory = path.dirname(fileURLToPath(import.meta.url));
const repositoryRoot = path.resolve(directory, "../..");
const fixture = (name) => JSON.parse(readFileSync(path.join(directory, `fixtures/${name}`), "utf8"));
const clone = (value) => JSON.parse(JSON.stringify(value));

test("manifest is deterministic and covers every required dataset scale", () => {
  const first = validateManifest(fixture("manifest.v1.json"), repositoryRoot);
  const second = validateManifest(fixture("manifest.v1.json"), repositoryRoot);
  assert.deepEqual(first, second);
  assert.deepEqual(first.datasets.map(({ id }) => id), ["repo-small", "repo-medium", "repo-large", "sessions-100", "runs-100", "runs-1000", "terminal-long", "token-stream-high-rate"]);
});

test("comparison emits complete provenance and only deterministic breaches fail", () => {
  const manifest = validateManifest(fixture("manifest.v1.json"), repositoryRoot);
  const results = compareManifest(manifest, provenance());
  assert.equal(formatFailures(results).length, 0);
  assert.equal(results.every((result) => result.commit === "abc123" && result.datasetVersion === 1), true);
  const dedicated = clone(manifest);
  dedicated.metrics.find(({ class: kind }) => kind === "dedicated-benchmark").measured = 999;
  assert.equal(formatFailures(compareManifest(dedicated, provenance())).length, 0);
});

test("known regression fails with actionable fields and leaves baseline immutable", () => {
  const manifest = validateManifest(fixture("manifest.v1.json"), repositoryRoot);
  const original = clone(manifest);
  const regressed = applyOverrides(manifest, fixture("known-regression.v1.json"));
  const failures = formatFailures(compareManifest(regressed, provenance()));
  assert.equal(failures.length, 1);
  assert.match(failures[0], /metric=mission-control\.query-count baseline=4 measured=1001 budget=4/);
  assert.deepEqual(manifest, original);
});

test("rejects traversal, duplicates, unsafe sizes, classes, units, and sensitive fields", () => {
  const cases = [
    (value) => { value.datasetRoot = "../outside"; },
    (value) => { value.datasets[0].fixturePath = "../escape.json"; },
    (value) => { value.datasets[1].id = value.datasets[0].id; },
    (value) => { value.datasets[0].scale.files = value.limits.maxFiles + 1; },
    (value) => { value.metrics[0].class = "timing-gate"; },
    (value) => { value.metrics[0].unit = "secrets"; },
    (value) => { value.metrics[1].id = value.metrics[0].id; },
  ];
  for (const mutate of cases) {
    const value = fixture("manifest.v1.json");
    mutate(value);
    assert.throws(() => validateManifest(value, repositoryRoot), /performance-/);
  }
  assert.throws(() => assertContentFree({ metricId: "safe", prompt: "secret" }), /performance-sensitive-field/);
  assert.throws(() => assertContentFree({ nested: { terminalContent: "secret" } }), /performance-sensitive-field/);
});

test("rejects malformed metadata and invalid result provenance before comparison", () => {
  const cases = [
    (value) => { delete value.schemaVersion; },
    (value) => { delete value.datasets[0].version; },
    (value) => { value.datasets = []; },
    (value) => { value.metrics[0].datasetId = "missing-dataset"; },
    (value) => { value.metrics[0].baseline = Number.NaN; },
    (value) => { value.metrics[0].headroom = ""; },
  ];
  for (const mutate of cases) {
    const value = fixture("manifest.v1.json");
    mutate(value);
    assert.throws(() => validateManifest(value, repositoryRoot), /performance-/);
  }

  const manifest = validateManifest(fixture("manifest.v1.json"), repositoryRoot);
  assert.throws(
    () => compareManifest(manifest, { platform: "win32", architecture: "x64", buildProfile: "test" }),
    /performance-string-bound: provenance.commit/,
  );
  assert.throws(
    () => compareManifest(manifest, { ...provenance(), credentials: "secret" }),
    /performance-sensitive-field/,
  );
});

test("loadJson reports malformed JSON without echoing parser input", async () => {
  const temporary = await mkdtemp(path.join(os.tmpdir(), "vanehub-performance-"));
  const malformed = path.join(temporary, "malformed.json");
  try {
    await writeFile(malformed, "{\"prompt\":\"secret\",", "utf8");
    await assert.rejects(() => loadJson(malformed), (error) => {
      assert.match(error.message, /performance-json-invalid: malformed-json/);
      assert.doesNotMatch(error.message, /secret/);
      return true;
    });
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
});

test("rejects oversized dataset and metric collections", () => {
  const tooManyDatasets = fixture("manifest.v1.json");
  tooManyDatasets.limits.maxDatasets = 1;
  assert.throws(() => validateManifest(tooManyDatasets, repositoryRoot), /performance-array-bound: datasets/);

  const tooManyMetrics = fixture("manifest.v1.json");
  tooManyMetrics.limits.maxMetrics = 1;
  assert.throws(() => validateManifest(tooManyMetrics, repositoryRoot), /performance-array-bound: metrics/);
});

function provenance() {
  return { commit: "abc123", platform: "win32", architecture: "x64", buildProfile: "test" };
}
