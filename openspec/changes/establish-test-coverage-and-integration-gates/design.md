## Context

The repository already runs Vitest, Rust tests, contract checks, and a Chromium Playwright suite in independent GitHub Actions jobs. Static inspection finds substantial test volume, including Agent Terminal application tests, MCP relay fixtures, migration tests, and SQLite rollback tests, but neither frontend nor Rust coverage is measured. Existing React component tests generally render server-side markup rather than exercising DOM interactions, while Playwright runs against the Web/mock adapter and cannot prove native lifecycle behavior.

The change crosses frontend test infrastructure, native test composition, and CI policy. It must preserve the React service boundary, keep SQLite and process ownership in Rust, remain deterministic on CI, and comply with the repository's immutable-Action and least-privilege workflow rules.

## Goals / Non-Goals

**Goals:**

- Produce reviewable frontend and native coverage reports that count untested production source.
- Enforce an explicit 80% line-coverage floor for designated critical Rust path groups and a measured non-regression baseline for the wider codebase.
- Add maintainable DOM interaction tests for Session category drag-and-drop and Prompt Hook editing.
- Prove the native Session and Agent Terminal lifecycle across context boundaries with real temporary SQLite persistence.
- Make tests, coverage, lint, Clippy, builds, and OpenSpec validation reliable merge gates.

**Non-Goals:**

- Add create or delete operations for registry-defined Agents.
- Require 80% coverage for every file or for the entire repository before measuring the current baseline.
- Replace Playwright, Vitest, Cargo test, or existing architecture/contract tests.
- Launch real Claude Code, OpenCode, Codex CLI, or Gemini CLI processes in deterministic CI tests.
- Add a third-party hosted coverage service or publish source, prompts, credentials, logs, or test databases externally.
- Treat coverage percentage as a substitute for scenario, transaction, security, or failure-path review.

## Decisions

1. Use Vitest's V8 coverage provider and explicitly enumerate frontend production source.

   Add a coverage script and configure V8 reporters for concise text, machine-readable JSON/LCOV, and local HTML. The include pattern covers `src/**/*.{ts,tsx}` so files never imported by tests appear as uncovered. Test files, declarations, generated artifacts, and explicit test utilities are excluded; the Web/mock adapter remains included because it is a supported runtime.

   V8 is selected because it is the native Vitest provider for the existing Node/Chromium toolchain and avoids a separate instrumentation stack. Istanbul is not needed unless a demonstrated V8 limitation blocks correct source mapping.

2. Introduce Testing Library with a DOM environment through a shared application test harness.

   Add React/DOM Testing Library, `user-event`, and a Vitest-compatible DOM implementation. A shared render helper assembles QueryClient, i18n, theme, and injected service doubles. Tests query roles, labels, values, and visible results rather than private state or component implementation.

   Drag-and-drop handler behavior is tested with a deterministic `DataTransfer` double in Vitest; at least one Playwright browser scenario retains responsibility for real browser drag behavior. This split keeps component tests fast without pretending a simulated DOM fully reproduces browser drag semantics.

3. Measure Rust with `cargo-llvm-cov` and keep the coverage policy in version control.

   CI installs a pinned `cargo-llvm-cov` release through a supply-chain-compliant, immutable mechanism. It emits machine-readable coverage plus a human-readable summary. A repository-owned policy maps normalized Rust source paths into the three critical groups and records wider-codebase baselines.

   The policy checker fails when a critical group is below 80%, a baseline regresses, a configured group matches no production file, or report parsing is incomplete. Path normalization must behave identically on Windows development machines and Linux CI.

   A global 80% threshold and per-file 80% threshold were rejected: the former has no measured feasibility yet, and the latter rewards trivial mapper/boilerplate tests while making platform-specific files disproportionately expensive.

4. Define coverage as line coverage for the initial native gate.

   Stable line coverage is used for the required 80% critical-path target. Function totals are reported for review but are not initially merge-blocking; unstable/nightly-only branch instrumentation is not introduced. Scenario requirements still mandate error, rollback, timeout, and lifecycle branches even when the numeric gate is line-based.

5. Implement native lifecycle integration inside the crate without widening production visibility.

   Add a crate-internal `#[cfg(test)]` native integration module, consistent with the existing contract and migration fixture modules. The harness assembles real `NativeDatabase` persistence and published sessions/agent-runtime application APIs while substituting deterministic process, terminal, clock, operation, and event ports.

   The lifecycle is Session creation, Agent Terminal open, running-state observation, terminal stop, and Session deletion. Agent registry entries are predefined tools, so Agent CRUD is intentionally not modeled. Tests assert persisted state and runtime cleanup without a Tauri window or installed provider CLI.

6. Extend transaction tests through failure injection at real SQLite boundaries.

   Existing transaction tests provide patterns using triggers and temporary databases. New or selected critical multi-write boundaries use the same technique to force a later statement to fail, then assert rollback and preservation of pre-existing data. Repository doubles remain appropriate for application sequencing, but they do not replace SQLite rollback proof.

7. Keep coverage jobs separate while avoiding duplicate test execution within each job.

   The frontend coverage command runs the Vitest suite once with coverage rather than running ordinary Vitest and covered Vitest sequentially. Native coverage runs in a separate Ubuntu job because instrumented Rust compilation is materially slower than normal check/test diagnostics. Existing Rust and browser jobs remain independently diagnosable.

   Coverage outputs upload with `if: always()` only after a report path exists, and a threshold failure remains the job result. No write permission or coverage-service credential is required.

8. Tighten warning policy in the same change.

   Rust CI runs Clippy for configured targets with `-D warnings`. Frontend lint runs with zero allowed warnings after existing warnings are resolved or rules are deliberately corrected. Warning suppression is not used to manufacture a green baseline.

## Risks / Trade-offs

- [Current coverage is below the intended policy] → Measure first, add targeted behavioral tests, commit truthful wider-codebase baselines, and enable all gates only when the same change is green.
- [Coverage grouping can be gamed or drift as files move] → Keep path groups reviewable in source control, fail empty matches, and require updates when critical modules move.
- [Instrumented Rust builds increase CI time] → Run native coverage in parallel on one supported host, cache only safe compiler artifacts if measurement proves it useful, and keep normal Rust diagnostics separate.
- [DOM drag simulation differs from a browser] → Limit Vitest to component/service semantics and retain a Playwright browser drag scenario.
- [Integration tests accidentally depend on local tools or credentials] → Use explicit fake process/terminal ports and temporary storage; fail tests that escape the harness.
- [Coverage reports expose sensitive fixture data] → Report only source locations and counts, use synthetic fixtures, and never include runtime logs, prompts, credentials, or user databases.
- [Zero-warning enforcement exposes existing debt] → Resolve warnings as an explicit task before switching the gate on; do not lower lint severity silently.

## Migration Plan

1. Add pinned coverage and Testing Library dependencies plus shared frontend/native test harnesses.
2. Measure and record initial frontend and native coverage without enabling thresholds.
3. Define and review critical Rust path groups, then add missing Agent, MCP, transaction, drag-and-drop, Prompt Hook, and lifecycle tests.
4. Enable the 80% critical Rust threshold, wider non-regression baselines, and zero-warning quality gates.
5. Run all repository validation locally and in the pull request, verify retained artifacts, and update main specifications after review.

Rollback removes the required coverage jobs and thresholds while retaining independent functional tests where possible. It must not weaken existing lint, build, Rust test, Playwright, contract, or OpenSpec checks.

## Open Questions

- What exact wider-codebase frontend and native baseline values will the first instrumented run establish?
- Which source patterns form the smallest stable critical Rust groups while still covering every Agent startup/terminal, MCP routing, and SQLite transaction behavior named by the specification?
