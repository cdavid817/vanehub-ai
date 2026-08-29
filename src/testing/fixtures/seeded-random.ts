/**
 * Deterministic PRNG and small text/id/time helpers shared by every large-scale fixture generator
 * under `src/testing/fixtures/`.
 *
 * Task 0.9 of `redesign-unified-workbench-ui` needs the same seed to reproduce byte-identical
 * output run after run, so structural-performance tests can compare DOM/query/render budgets
 * across CI runs without a shifting baseline. Nothing here reads `Math.random()` or `Date.now()`.
 */

export type SeededRandom = () => number;

/** mulberry32: small, fast, good enough for fixture shaping -- not for anything security-sensitive. */
export function createSeededRandom(seed: number): SeededRandom {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** The one non-derived seed. Every generator either uses this or a caller-supplied override. */
export const DEFAULT_SEED = 424242;

/** Fixed literal instants, not `Date.now()`: the same two points in time on every machine. */
export const FIXTURE_RANGE_START_MS = Date.parse("2024-06-01T00:00:00Z");
export const FIXTURE_RANGE_END_MS = Date.parse("2026-08-30T00:00:00Z");

export function nextInt(rng: SeededRandom, minInclusive: number, maxExclusive: number): number {
  return minInclusive + Math.floor(rng() * (maxExclusive - minInclusive));
}

export function chance(rng: SeededRandom, probability: number): boolean {
  return rng() < probability;
}

export function pick<T>(rng: SeededRandom, items: readonly T[]): T {
  return items[nextInt(rng, 0, items.length)];
}

export function pickWeighted<T>(rng: SeededRandom, weighted: ReadonlyArray<readonly [T, number]>): T {
  const total = weighted.reduce((sum, [, weight]) => sum + weight, 0);
  let roll = rng() * total;
  for (const [value, weight] of weighted) {
    roll -= weight;
    if (roll <= 0) return value;
  }
  return weighted[weighted.length - 1][0];
}

/**
 * `bucketCount` integers within `[min, max]` that sum to exactly `total`.
 *
 * Backs every "N items split unevenly across M buckets but the grand total must be exact" need in
 * this directory -- attempts per evaluation arena, checks per attempt -- so a headline count from
 * the OpenSpec task is exact rather than "approximately".
 */
export function distributeExact(rng: SeededRandom, bucketCount: number, total: number, min: number, max: number): number[] {
  if (bucketCount <= 0) return [];
  const values = Array.from({ length: bucketCount }, () => nextInt(rng, min, max + 1));
  let diff = total - values.reduce((sum, value) => sum + value, 0);
  // Deterministic full sweeps, not random probing: a pass touches every bucket once, so it always
  // makes maximal progress. `progressed` only stays false once every bucket is pinned at the bound
  // `diff`'s sign needs, which is exactly when `total` is infeasible for `[bucketCount*min,
  // bucketCount*max]` -- at that point stopping is correct, not a missed budget.
  let progressed = true;
  while (diff !== 0 && progressed) {
    progressed = false;
    for (let index = 0; index < bucketCount && diff !== 0; index += 1) {
      if (diff > 0 && values[index] < max) {
        values[index] += 1;
        diff -= 1;
        progressed = true;
      } else if (diff < 0 && values[index] > min) {
        values[index] -= 1;
        diff += 1;
        progressed = true;
      }
    }
  }
  return values;
}

/** Monotonic counter, not RNG-derived: uniqueness holds regardless of RNG state or call order. */
export function createIdFactory(prefix: string): () => string {
  let counter = 0;
  return () => {
    counter += 1;
    return `${prefix}-${counter.toString().padStart(6, "0")}`;
  };
}

export function isoTimestamp(rng: SeededRandom, startMs: number, endMs: number): string {
  return new Date(startMs + Math.floor(rng() * Math.max(1, endMs - startMs))).toISOString();
}

/** An ISO timestamp offset from `iso` by an amount in `[minMs, maxMs]`. Either bound may be negative. */
export function offsetTimestamp(iso: string, minMs: number, maxMs: number, rng: SeededRandom): string {
  const base = Date.parse(iso);
  return new Date(base + nextInt(rng, minMs, maxMs + 1)).toISOString();
}

const WORD_BANK = [
  "alpha", "bridge", "cascade", "dataset", "evidence", "framework", "gateway", "harbor", "insight", "journal",
  "kernel", "ledger", "migration", "nucleus", "orchestrate", "pipeline", "quota", "registry", "synthesis", "telemetry",
  "undo", "validate", "workspace", "yield", "zenith", "archive", "boundary", "checkpoint", "delta", "envelope",
  "fragment", "governance", "horizon", "inventory", "junction", "keystone", "lattice", "milestone", "notebook", "outcome",
  "payload", "quarantine", "reservoir", "scaffold", "threshold", "upstream", "vantage", "widget", "transcript", "runbook",
];

export function words(rng: SeededRandom, minWords: number, maxWords: number): string {
  const count = nextInt(rng, minWords, maxWords + 1);
  return Array.from({ length: count }, () => pick(rng, WORD_BANK)).join(" ");
}

function titleCase(text: string): string {
  return text.length === 0 ? text : text.charAt(0).toUpperCase() + text.slice(1);
}

export function title(rng: SeededRandom, minWords = 2, maxWords = 6): string {
  return titleCase(words(rng, minWords, maxWords));
}

/** Returns `long()` with low, deterministic probability -- the "occasional very long value" every domain needs for truncation stress tests. */
export function maybeLong(rng: SeededRandom, normal: () => string, long: () => string, probability = 0.03): string {
  return chance(rng, probability) ? long() : normal();
}

/** A short 1-3 segment path normally, or a deep 14-24 segment one when `long` is true. */
export function fixturePath(rng: SeededRandom, long = false): string {
  const segments = Array.from({ length: long ? nextInt(rng, 14, 24) : nextInt(rng, 1, 3) }, () => pick(rng, WORD_BANK));
  return `D:/workspace/${segments.join("/")}`;
}
