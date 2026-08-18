import { readFile } from "node:fs/promises";
import path from "node:path";

export const METRIC_CLASSES = new Set(["deterministic-gate", "dedicated-benchmark", "informational-telemetry"]);
export const UNITS = new Set(["allocations", "batches", "bytes", "count", "items", "milliseconds", "operations", "percent", "queries", "rows"]);
const DIRECTIONS = new Set(["lower", "upper"]);
const DATASET_KINDS = new Set(["repository", "runner", "runs", "sessions", "stream", "terminal"]);
const SENSITIVE_FIELDS = /^(content|credential|credentials|environment|fileContent|message|messages|prompt|prompts|rawError|rawFrame|response|terminalContent|toolArguments|toolResults|unrestrictedPath)$/i;
const SAFE_ID = /^[a-z0-9]+(?:[.-][a-z0-9]+)*$/;

export async function loadJson(filePath) {
  let parsed;
  try {
    parsed = JSON.parse(await readFile(filePath, "utf8"));
  } catch (error) {
    throw new Error(`performance-json-invalid: ${safeError(error)}`, { cause: error });
  }
  return parsed;
}

export function validateManifest(input, repositoryRoot) {
  const manifest = object(input, "manifest");
  integer(manifest.schemaVersion, "schemaVersion", 1, 1);
  const limits = object(manifest.limits, "limits");
  const maxDatasets = integer(limits.maxDatasets, "limits.maxDatasets", 1, 64);
  const maxMetrics = integer(limits.maxMetrics, "limits.maxMetrics", 1, 256);
  const maxFiles = integer(limits.maxFiles, "limits.maxFiles", 1, 100_000);
  const maxBytes = integer(limits.maxBytes, "limits.maxBytes", 1, 1_073_741_824);
  const maxItems = integer(limits.maxItems, "limits.maxItems", 1, 1_000_000);
  const datasetRoot = safeFixtureRoot(repositoryRoot, string(manifest.datasetRoot, "datasetRoot"));
  const datasets = array(manifest.datasets, "datasets", maxDatasets);
  const datasetIds = new Set();
  const normalizedDatasets = datasets.map((value, index) => {
    const dataset = object(value, `datasets[${index}]`);
    const id = idValue(dataset.id, `datasets[${index}].id`);
    unique(datasetIds, id, "dataset");
    const kind = enumValue(dataset.kind, DATASET_KINDS, `datasets[${index}].kind`);
    const scale = validateScale(object(dataset.scale, `datasets[${index}].scale`), { maxBytes, maxFiles, maxItems });
    if (kind === "runner") validateRunnerScale(scale);
    const fixturePath = safeFixturePath(datasetRoot, string(dataset.fixturePath, `datasets[${index}].fixturePath`));
    return { id, version: integer(dataset.version, `datasets[${index}].version`, 1, 1_000_000), kind, seed: integer(dataset.seed, `datasets[${index}].seed`, 1, 2_147_483_647), fixturePath, scale };
  });
  const metrics = array(manifest.metrics, "metrics", maxMetrics);
  const metricIds = new Set();
  const normalizedMetrics = metrics.map((value, index) => {
    const metric = object(value, `metrics[${index}]`);
    const id = idValue(metric.id, `metrics[${index}].id`);
    unique(metricIds, id, "metric");
    const datasetId = idValue(metric.datasetId, `metrics[${index}].datasetId`);
    if (!datasetIds.has(datasetId)) throw new Error(`performance-unknown-dataset: ${datasetId}`);
    const baseline = finite(metric.baseline, `metrics[${index}].baseline`);
    const budget = finite(metric.budget, `metrics[${index}].budget`);
    const measured = finite(metric.measured, `metrics[${index}].measured`);
    const headroom = string(metric.headroom, `metrics[${index}].headroom`, 256);
    if (!headroom.trim()) throw new Error(`performance-headroom-required: ${id}`);
    return { id, datasetId, class: enumValue(metric.class, METRIC_CLASSES, `metrics[${index}].class`), unit: enumValue(metric.unit, UNITS, `metrics[${index}].unit`), direction: enumValue(metric.direction, DIRECTIONS, `metrics[${index}].direction`), baseline, budget, measured, headroom };
  });
  return { schemaVersion: 1, datasetRoot, limits: { maxDatasets, maxMetrics, maxFiles, maxBytes, maxItems }, datasets: normalizedDatasets, metrics: normalizedMetrics };
}

export function applyOverrides(manifest, input) {
  const fixture = object(input, "override fixture");
  integer(fixture.schemaVersion, "schemaVersion", 1, 1);
  const overrides = array(fixture.overrides, "overrides", manifest.metrics.length);
  const byId = new Map(manifest.metrics.map((metric) => [metric.id, { ...metric }]));
  const seen = new Set();
  for (const [index, value] of overrides.entries()) {
    const override = object(value, `overrides[${index}]`);
    const metricId = idValue(override.metricId, `overrides[${index}].metricId`);
    unique(seen, metricId, "override");
    const metric = byId.get(metricId);
    if (!metric) throw new Error(`performance-unknown-metric: ${metricId}`);
    metric.measured = finite(override.measured, `overrides[${index}].measured`);
  }
  return { ...manifest, metrics: [...byId.values()] };
}

export function compareManifest(manifest, provenance) {
  validateProvenance(provenance);
  return manifest.metrics.map((metric) => {
    const dataset = manifest.datasets.find((candidate) => candidate.id === metric.datasetId);
    const breached = metric.direction === "upper" ? metric.measured > metric.budget : metric.measured < metric.budget;
    const enforced = metric.class === "deterministic-gate";
    return {
      schemaVersion: 1,
      ...provenance,
      datasetId: dataset.id,
      datasetVersion: dataset.version,
      metricId: metric.id,
      metricClass: metric.class,
      measured: metric.measured,
      unit: metric.unit,
      baseline: metric.baseline,
      budget: metric.budget,
      delta: metric.measured - metric.baseline,
      outcome: breached && enforced ? "failed" : breached ? "evidence-only" : "passed",
    };
  });
}

export function formatFailures(results) {
  return results.filter((result) => result.outcome === "failed").map((result) => `metric=${result.metricId} baseline=${result.baseline} measured=${result.measured} budget=${result.budget} delta=${result.delta} unit=${result.unit} dataset=${result.datasetId}@${result.datasetVersion} platform=${result.platform}/${result.architecture} profile=${result.buildProfile}`);
}

export function assertContentFree(value) {
  visit(value, "result");
}

function visit(value, location) {
  if (Array.isArray(value)) return value.forEach((entry, index) => visit(entry, `${location}[${index}]`));
  if (!value || typeof value !== "object") return;
  for (const [key, nested] of Object.entries(value)) {
    if (SENSITIVE_FIELDS.test(key)) throw new Error(`performance-sensitive-field: ${location}.${key}`);
    visit(nested, `${location}.${key}`);
  }
}

function validateProvenance(value) {
  const provenance = object(value, "provenance");
  for (const field of ["commit", "platform", "architecture", "buildProfile"]) string(provenance[field], `provenance.${field}`, 128);
  assertContentFree(provenance);
}

function validateScale(scale, limits) {
  const normalized = {};
  for (const [key, value] of Object.entries(scale)) {
    const limit = key.toLowerCase().includes("bytes") ? limits.maxBytes : key === "files" ? limits.maxFiles : limits.maxItems;
    normalized[key] = integer(value, `scale.${key}`, 0, limit);
  }
  if (Object.keys(normalized).length === 0) throw new Error("performance-empty-scale");
  return normalized;
}

function validateRunnerScale(scale) {
  for (const key of ["activeHandles", "localHandles", "sshHandles", "sshTargets", "maxPerSshTarget", "eventQueueItems", "eventChunkBytes", "reconnectAttempts", "channelsPerRun"]) {
    if (!(key in scale)) throw new Error(`performance-runner-scale-required: ${key}`);
  }
  if (scale.activeHandles !== scale.localHandles + scale.sshHandles) throw new Error("performance-runner-handle-sum");
  if (scale.activeHandles > 32 || scale.localHandles > 24 || scale.sshHandles > 24) throw new Error("performance-runner-handle-bound");
  if (scale.maxPerSshTarget > 8 || scale.sshHandles > scale.sshTargets * 8) throw new Error("performance-runner-target-bound");
  if (scale.eventQueueItems > 256 || scale.eventChunkBytes > 8192) throw new Error("performance-runner-output-bound");
  if (scale.reconnectAttempts > 1 || scale.channelsPerRun > 1) throw new Error("performance-runner-channel-bound");
}

function safeFixtureRoot(repositoryRoot, relative) {
  if (path.isAbsolute(relative)) throw new Error("performance-absolute-fixture-root");
  const root = path.resolve(repositoryRoot);
  const resolved = path.resolve(root, relative);
  if (resolved !== root && !resolved.startsWith(`${root}${path.sep}`)) {
    throw new Error("performance-fixture-root-traversal");
  }
  return resolved;
}

function safeFixturePath(root, relative) {
  if (path.isAbsolute(relative)) throw new Error("performance-absolute-fixture-path");
  const resolved = path.resolve(root, relative);
  if (resolved !== root && !resolved.startsWith(`${root}${path.sep}`)) throw new Error("performance-fixture-traversal");
  return resolved;
}

function object(value, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(`performance-object-required: ${label}`);
  return value;
}

function array(value, label, maximum) {
  if (!Array.isArray(value) || value.length === 0 || value.length > maximum) throw new Error(`performance-array-bound: ${label}`);
  return value;
}

function string(value, label, maximum = 512) {
  if (typeof value !== "string" || value.length === 0 || value.length > maximum) throw new Error(`performance-string-bound: ${label}`);
  return value;
}

function idValue(value, label) {
  const id = string(value, label, 96);
  if (!SAFE_ID.test(id)) throw new Error(`performance-invalid-id: ${label}`);
  return id;
}

function integer(value, label, minimum, maximum) {
  if (!Number.isInteger(value) || value < minimum || value > maximum) throw new Error(`performance-integer-bound: ${label}`);
  return value;
}

function finite(value, label) {
  if (typeof value !== "number" || !Number.isFinite(value)) throw new Error(`performance-number-required: ${label}`);
  return value;
}

function enumValue(value, allowed, label) {
  if (!allowed.has(value)) throw new Error(`performance-unknown-value: ${label}`);
  return value;
}

function unique(values, value, label) {
  if (values.has(value)) throw new Error(`performance-duplicate-${label}: ${value}`);
  values.add(value);
}

function safeError(error) {
  return error instanceof SyntaxError ? "malformed-json" : "read-failed";
}
