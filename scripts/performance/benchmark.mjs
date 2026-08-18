import { performance } from "node:perf_hooks";

export function measureSyntheticDatasets(manifest) {
  const measurements = new Map();
  for (const dataset of manifest.datasets) {
    if (dataset.kind === "repository") measurements.set(`repository.${dataset.id}`, sample(() => repositoryWork(dataset)));
    if (dataset.kind === "runs") measurements.set(`runs.${dataset.id}`, sample(() => runWork(dataset)));
    if (dataset.kind === "runner") measurements.set(`runner.${dataset.id}`, sample(() => runnerWork(dataset)));
    if (dataset.kind === "terminal") measurements.set(`terminal.${dataset.id}`, sample(() => terminalWork(dataset)));
    if (dataset.kind === "stream") measurements.set(`stream.${dataset.id}`, sample(() => streamWork(dataset)));
  }
  return [...measurements].map(([id, value]) => ({ id, ...value }));
}

function runnerWork(dataset) {
  const handles = Array.from({ length: dataset.scale.activeHandles }, (_, index) => ({
    kind: index < dataset.scale.localHandles ? 1 : 2,
    target: index < dataset.scale.localHandles ? 0 : index % Math.max(1, dataset.scale.sshTargets),
  }));
  let checksum = 0;
  for (const handle of handles) checksum = hash(checksum + handle.kind + handle.target + dataset.seed);
  handles.length = 0;
  return checksum;
}

function sample(work) {
  const samples = [];
  let checksum = 0;
  for (let index = 0; index < 7; index += 1) {
    const started = performance.now();
    checksum ^= work();
    samples.push(performance.now() - started);
  }
  samples.sort((left, right) => left - right);
  return { p50Milliseconds: round(samples[3]), p95Milliseconds: round(samples[6]), samples: samples.length, checksum };
}

function repositoryWork(dataset) {
  const size = Math.min(dataset.scale.symbols, 65_536);
  const values = Array.from({ length: size }, (_, index) => hash(dataset.seed + index));
  values.sort((left, right) => left - right);
  return values[Math.floor(values.length / 2)] ?? 0;
}

function runWork(dataset) {
  let state = 0;
  for (let run = 0; run < dataset.scale.items; run += 1) {
    for (let event = 0; event < dataset.scale.eventsPerRun; event += 1) state = hash(state + run + event + dataset.seed);
  }
  return state;
}

function terminalWork(dataset) {
  let matches = 0;
  for (let chunk = 0; chunk < dataset.scale.chunks; chunk += 1) matches += hash(chunk + dataset.seed) % 31 === 0 ? 1 : 0;
  return matches;
}

function streamWork(dataset) {
  let batches = 0;
  for (let index = 0; index < dataset.scale.items; index += dataset.scale.eventsPerFrame) batches += 1;
  return batches;
}

function hash(value) {
  let result = value | 0;
  result = Math.imul(result ^ (result >>> 16), 0x45d9f3b);
  result = Math.imul(result ^ (result >>> 16), 0x45d9f3b);
  return (result ^ (result >>> 16)) >>> 0;
}

function round(value) {
  return Math.round(value * 1000) / 1000;
}
