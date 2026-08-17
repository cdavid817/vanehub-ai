## Context

The current repository already has optimized release/chunk budgets, blocking-work isolation, bounded query tests, chat and terminal frame coalescing, a deterministic Context Engine quality corpus, Mission Control large-history coverage, LSP fixtures, and terminal capture/search limits. The completed active change `harden-runtime-lifecycle-and-boundaries` adds batched relationship reads, short registry lock scopes, and bounded terminal presentation; this change consumes that baseline and does not reopen it.

Baseline measurements taken before this design on commit `f3441915`, Windows x86_64, debug/test profile:

- Context corpus: 8 cases, 18 candidate operations, 0 budget overflow, observed ranking/selection duration 1.159 ms after compilation. The existing hard gate is `operations <= 32`; elapsed time is informational.
- Mission Control: indexed newest query, invalid-cursor no-query behavior, and bounded safe projection passed against the existing large-history fixture (4 focused Rust tests, 2.21 s total test execution).
- Frontend event coalescing: 9 focused Vitest tests passed; test execution was 23 ms and is not a budget.
- Existing long-list Playwright coverage: 500+ Prompt Hooks render fewer than 30 rows and 600 logs render fewer than 50 rows; both scenarios passed. Existing React `flushSync` console noise was observed and is recorded as an unrelated follow-up rather than expanded into this change.

The roadmap requires measurements wider than these isolated assertions, but shared CI cannot reliably enforce host timing, CPU, or memory. No public frontend performance API currently exists, and the optional Diagnostics UI is not required for acceptance.

## Goals / Non-Goals

**Goals:**

- Give contributors one repeatable command and normalized evidence format for runtime budgets.
- Turn existing structural limits into versioned datasets and deterministic gates, then add missing Run, Context, LSP, terminal, persistence, parser, comparator, and negative coverage.
- Preserve metric provenance and privacy while allowing dedicated P50/P95, throughput, memory, cold-start, TTI, idle CPU, and long-task evidence.
- Make 100/1,000 Run histories, long terminal output, high-rate token streams, and small/medium/large synthetic repositories reproducible.

**Non-Goals:**

- Add a performance bounded context, database tables, Tauri commands, React service methods, or a Diagnostics UI.
- Replace product Eval/Benchmark semantics or implement Hybrid Local Model Runtime, Runner Abstraction, or later roadmap items.
- Claim macOS/Linux evidence from a Windows run or turn model/network latency into a CI gate.
- Fix unrelated console warnings or tune performance without evidence of a breached budget.

## Decisions

1. **Use repository tooling as the harness coordinator.** A dependency-free Node entry point under `scripts/performance/` will validate manifests/results, compare budgets, print human-readable failures, and emit JSON evidence. `package.json` exposes deterministic check and dedicated measurement commands. This reuses npm as the repository task boundary and avoids a runtime subsystem or production dependency. A native-only Criterion suite was rejected because it cannot coordinate frontend, fixture metadata, or platform evidence.

2. **Keep measurements beside owning implementations.** Rust tests remain in their existing bounded contexts; frontend coalescing and list tests remain in Vitest/Playwright. The coordinator consumes normalized result fixtures and commands rather than importing private domain modules across contexts. No React component calls Tauri, and no new adapter method is needed because there is no product UI.

3. **Version a compact manifest and generated synthetic metadata.** The manifest declares dataset ids/versions, fixture roots, scale parameters, metric class/unit/direction, baseline, and justified budget. Synthetic content is generated deterministically from declared counts and seeds, so small/medium/large repositories, 100 sessions, 100/1,000 Runs, long terminal, and high-rate streams do not add huge checked-in blobs. Paths are canonicalized beneath the fixture root and counts/bytes are capped before execution.

4. **Separate hard gates from evidence.** `deterministic-gate` covers query count, loaded rows, item/byte/buffer/chunk limits, batch count, and prohibited synchronous/N+1 structures. `dedicated-benchmark` records latency/throughput/memory and supports relative regression budgets derived from a platform/profile baseline. `informational-telemetry` records cold start, TTI, idle memory/CPU, and main-thread long tasks. Only deterministic failures set a nonzero exit code in shared CI.

5. **Derive initial thresholds from current evidence and contract caps.** Existing bounds are retained where already normative: Context operations baseline 18 with hard ceiling 32; Mission Control page/query behavior remains constant and page-bounded; prompt/log rendered rows retain current limits; chat/Run bursts coalesce to bounded batches; terminal/LSP values use their current declared caps. New negative fixtures exceed exactly one declared budget so parser and comparator failures are stable. Dedicated timing budgets are stored only after matching platform/profile samples and use justified relative headroom.

6. **Treat result files as metadata-only artifacts.** Results contain commit, platform, architecture, profile, dataset version, metric id/class, numeric values, unit, baseline, budget, delta, outcome, and bounded correlations. Content, prompts, credentials, terminal bytes, raw errors, and unrestricted paths are rejected. Product diagnostics continue through the existing operations/unified-log contracts; harness output itself stays in build/test artifacts and does not create feature-local runtime logs.

7. **Prove structural coverage at owning boundaries.** Context tests expose phase operation and occupancy counts; Run/Mission Control tests exercise 100/1,000 histories and constant query/page behavior; token events assert bounded reducer/coalescer traversals; terminal tests assert chunk/buffer/search bounds; code-intelligence tests assert repository, queue, response caps and preserve unavailable semantics. Dedicated elapsed measurements accompany but do not replace these assertions.

8. **Record platform status literally.** The evidence summary uses `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` per operating system. This worktree can produce Windows evidence only; macOS and Linux remain `NOT RUN` unless corresponding native jobs are actually executed and their artifacts inspected.

## Risks / Trade-offs

- [A manifest can drift from implementation limits] -> Contract tests load the same versioned manifest and assert known metric ids, units, datasets, and bounds; reviewable baseline changes must include justification.
- [Synthetic workloads miss real-device bottlenecks] -> Keep dedicated and informational capture fields and document that they supplement, not replace, deterministic gates.
- [Performance commands become too slow for contributors] -> Split fast `performance:check` from opt-in `performance:benchmark`; CI hard gates use bounded fixtures and focused tests.
- [Sensitive data leaks into evidence] -> Use allowlisted schemas, bounded strings, fixture-root path validation, negative security tests, and no raw workload content in results.
- [Existing active hardening change overlaps] -> Depend on its completed batching/coalescing behavior and add only measurement/governance deltas; do not edit its artifacts or duplicate its tasks.

## Migration Plan

No user-data or database migration is required. Add the manifest, fixtures, coordinator, tests, and documentation; run deterministic gates before dedicated measurements. Rollback removes repository tooling and tests without changing runtime contracts or stored data. After verification, archive through OpenSpec so the eight deltas merge into their existing main capabilities.
