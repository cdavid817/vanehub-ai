# Runtime performance budgets

VaneHub keeps runtime performance evidence reproducible without turning noisy shared-runner timing into a merge gate. The harness coordinates metadata and comparison; measurements remain beside the frontend or native bounded context that owns the behavior.

## Commands

```bash
npm run performance:check
npm run performance:benchmark
```

`performance:check` validates the versioned manifest, runs parser/comparator security tests, and enforces deterministic budgets. Its JSON output identifies the commit, platform, architecture, build profile, dataset version, metric, baseline, budget, delta, and outcome.

`performance:benchmark` runs bounded synthetic workloads and reports sample count plus P50/P95 latency. It is dedicated evidence, not an absolute shared-CI timing gate. Use the same commit, platform, architecture, build profile, power state, and dataset version when comparing results.

## Metric classes

| Class | Examples | Shared-CI behavior |
| --- | --- | --- |
| `deterministic-gate` | query count, loaded rows, chunks, bytes, items, batches, operations | Fails when the declared upper or lower bound is breached |
| `dedicated-benchmark` | latency, throughput, memory | Recorded for matching environments; never fails shared CI solely because of elapsed time |
| `informational-telemetry` | cold start, TTI, idle memory/CPU, main-thread long tasks | Recorded from real devices when available; not a PR hard gate |

## Datasets and safety

The v1 manifest declares deterministic seeds and bounded scales for small, medium, and large repositories, 100 sessions, 100 and 1,000 Runs, long terminal output, and a high-rate token stream. Large blobs are generated from scale metadata rather than committed. Fixture paths must remain under the declared root; dataset counts and bytes are validated before work begins.

Evidence is metadata-only. The schema rejects prompts, messages, responses, tool arguments/results, credentials, environment values, raw frames/errors, terminal or file content, and unrestricted paths. Runtime diagnostics continue to use the unified logging service; harness output belongs to test/CI artifacts and is not a feature-local runtime log.

## Baseline updates

Change a baseline or budget only with matching measurement evidence and a short headroom justification in `scripts/performance/fixtures/manifest.v1.json`. Structural limits should reflect an owning contract or observed current baseline plus defensible capacity. Dedicated relative thresholds require samples from the same platform and build profile.

The checked-in negative fixture intentionally raises Mission Control query count from 4 to 1,001. Unit coverage must continue to reject it and prove that applying the override does not mutate the accepted manifest.

## Platform reporting

Report native status separately as `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN`. Never infer macOS or Linux behavior from Windows evidence, or one architecture from another.
