# Tasks

## 1. Specification deltas

- [x] 1.1 Write the `user-guide-documentation` delta: replace the authoritative-plus-transition model with two complete equivalent guides, retire the labeling requirement, and extend the coverage requirement to both guides symmetrically.
- [x] 1.2 Write the `native-developer-documentation` delta: govern the developer guide as a bilingual deliverable with equivalent chapters and sections, add reachability to the validated pipeline, and require reachability to be enforced automatically.
- [x] 1.3 Run `openspec validate restore-documentation-spec-conformance --strict`. Passes.
- [x] 1.4 Confirm no scenario is silently dropped. The validator now rejects a `MODIFIED` block that omits an existing scenario, which is what forced the three obsolete requirements into `REMOVED` plus `ADDED` rather than `MODIFIED`. The archive merge itself still runs at archive time.

## 2. A1 — English user guide reconciles toward Simplified Chinese

- [x] 2.1 Remove the `**Status: ...**` opening line from the 23 English chapters that carried one (24 occurrences; `getting-started.md` had two).
- [x] 2.2 Remove dedicated Web/mock sections and browser-preview prose from the 25 English chapters that carried them, rewriting the surrounding prose so nothing is left dangling.
- [x] 2.3 Keep genuine environment dependencies as prose at the step they affect — `- **Desktop only.**` bullets survive, the browser-preview clause after them does not.
- [x] 2.4 Delete `docs/user-guide/en/src/runtime-labels.md` and its `SUMMARY.md` entry.
- [x] 2.5 Remove every cross-reference to `runtime-labels.md` from chapters and `index.md`.
- [x] 2.6 Confirm both `SUMMARY.md` files list the same chapters: 35 entries each, 36 chapters each.
- [x] 2.7 Clean the five residual Web/mock mentions the Chinese guide still carried in `multi-agent-workflow.md`, `plugin-integration.md`, and `tooling.md`.
- [x] 2.8 Bring the two user guides to equal heading counts in every chapter, which required adding `## Traditional uses`, `## Why it is essential in the Agent era`, and `### Things to watch once you have several` to the English `worktree.md`; `## What LSP is`, `### What LSP covers`, and `## Why an Agent needs LSP` to the English `lsp-code-intelligence.md`; `## Reading the log files` with three subsections to the English `troubleshooting.md`; and folding the emptied `## Runtime differences` section of `skill-management.md` into the section above it.

## 3. B — English developer guide reaches the Chinese section structure

- [x] 3.1 `repository-orientation.md`: bounded-context inventory for all seven core contexts, the extension-context list, and the request path through the layers. 32 → 190 lines, 3 diagrams.
- [x] 3.2 `permission-model.md`: decision flow and states, plus key types and constants.
- [x] 3.3 `agent-lifecycle.md`: registry-to-launch flow, key types and constants, and single-Agent runtime shapes.
- [x] 3.4 `loop-and-plan-runtime.md`: Loop Engineering background, iteration state machine, and key types and constants.
- [x] 3.5 `runtime-boundaries.md`: runtime selection and adapters, key files and contracts, and child-process communication.
- [x] 3.6 `skill-management.md`: the Skill / MCP / function-calling layering, configuration drift and readiness, key types, and the unified CLI plus OnePiece architecture.
- [x] 3.7 `session-recovery.md`: the recovery flow and key types and constants.
- [x] 3.8 `persistence-and-logging.md`: SQLite ownership and migrations, unified logging architecture, and key constants and redaction rules.
- [x] 3.9 `mcp-tools.md`: MCP protocol background, transports and the relay, key constants, and the unified architecture.
- [x] 3.10 `im-connectors.md`: message flow and routing, plus key constants and credentials.
- [x] 3.11 `tree-sitter-code-indexing.md`: the index build pipeline and key constants and admission.
- [x] 3.12 `testing-and-release.md`: test tiers, the release process, and key scripts and commands.
- [x] 3.13 `usage-statistics.md`: collection paths and accounting quality, plus key types and collection details.
- [x] 3.14 `cross-session-memory.md`: memory storage and production paths, plus key types and constants.
- [x] 3.15 `retrieval.md`: the retrieval flow and degradation, plus key types and constants.
- [x] 3.16 `tool-registry.md`: the tool call loop, interface format translation, and the fixed catalog and its boundaries.
- [x] 3.17 `lsp-code-intelligence.md`: process state machine and request sequence, why this is safe, and key types and constants.
- [x] 3.18 `multi-agent-group-chat.md`: both directions. English gained `## Why Multi-Agent at all` and `## Runtime shapes of a seat's Agent`; Chinese gained the user-message routing and seat-id attribution subsections, live desktop verification, environment findings, and the no-orchestrator rationale.
- [x] 3.19 Verify every added section against source. Result: 40 chapters, 0 heading-count mismatches, English 4850 lines against Chinese 4733, and 42 Mermaid diagrams on each side, up from 14 on the English one.

### Defects found while verifying against source

- [x] 3.20 `loop-and-plan-runtime.md` (zh) claimed `Deciding` is not a `LoopRunPhase`. `loop_engineering.rs:53` lists it as a variant. Corrected.
- [x] 3.21 `loop-and-plan-runtime.md` (zh) listed three `LoopLimits` fields; the struct has five. Corrected, with `max_iterations` documented as accepting `1..=20`.
- [x] 3.22 `persistence-and-logging.md` (zh) stated "79 sequential migrations"; `EXPECTED_MIGRATIONS` holds 94. Both languages now point at the constant and state no count, because the number is allocated across branches and rots.
- [x] 3.23 `runtime-boundaries.md` (zh) cited `agent-service.ts` line 214 for `AgentService`; it is at line 134. The line number is removed rather than corrected.
- [x] 3.24 `tool-registry.md` (zh) cited `api_process_adapter.rs`; the file is `api_process_adapter/mod.rs`. Corrected in both languages, and the conditionally injected tools are now named.
- [x] 3.25 `multi-agent-group-chat.md` (en) still described the human-decision e2e coverage as an open gap after `8a34b3a1` added the spec that closes it. Corrected.
- [x] 3.26 Both `testing-and-release.md` chapters and `lsp-code-intelligence.md` listed `cargo clippy --manifest-path ... --all-targets`, the weaker variant `AGENTS.md` warns against. Corrected to `--workspace`, with the reason stated.
- [x] 3.27 Both `troubleshooting.md` chapters stated that logs deliberately carry no execution identifiers and cannot be searched by trace id. `runtime_support.rs:378-385` inserts `runId`, `traceId`, and `spanId` into the log context. Corrected in both languages, including the developer-guide cross-reference.

## 4. C — Reachability gate and orphan disposition

- [x] 4.1 Add a reachability pass to `scripts/validate-docs.mjs` that traverses from the four `SUMMARY.md` files, the repository entry points, and `openspec/specs/**/spec.md`.
- [x] 4.2 Add unit coverage in `scripts/validate-docs.node-test.mjs`: transitive reachability, an unreached document, a mutually linked island, a root with no inbound link, and a cycle a root does reach. The docs unit suite is 40 tests, up from 35.
- [x] 4.3 Link `docs/provider-sdk/` (5 documents), `docs/desktop-release-verification.md`, `docs/runtime-performance-budgets.md`, and `docs/cli-agent-global-configuration.md` from a reference section in both developer-guide `index.md` files.
- [x] 4.4 Move `docs/architecture/skill-tool-runtime-security.md` into both developer guides as a snapshot-labelled chapter, add it to both `SUMMARY.md` files, update both `tool-registry.md` links, and remove `docs/architecture/`.
- [x] 4.5 Remove `docs/agent-platform-roadmap/` (14 documents and `manifest.json`).
- [x] 4.6 Remove `docs/reports/` (2 dated verification reports).
- [x] 4.7 Remove `docs/ux-audit-report.md` and `docs/ux-optimization-summary.md`.
- [x] 4.8 Run the check: 176 of 176 documents reachable, 0 unreachable.
- [x] 4.9 Confirm the gate fails on a regression. A standalone orphan and a mutually linked pair were both reported, then removed.
- [x] 4.10 Remove the orphaned `plan-center-en.png` and `plan-center-zh-CN.png`, left behind when the Plan Center was retired: in no inventory entry and referenced by no chapter.

## 5. D — Repository entry points

- [x] 5.1 Correct the command examples in `CONTRIBUTING.md` to `cargo clippy --workspace --all-targets -- -D warnings` and the `--workspace` forms of `check` and `test`.
- [x] 5.2 State why `--workspace` matters, and that `cargo fmt` is the exception that does use `--manifest-path`.

## 6. Media

- [x] 6.1 Add six capture scenarios to `tests/docs/documentation-screenshots.spec.ts`: `settings-code-intelligence`, `settings-about`, `todo-board`, `goal-center`, `evaluations`, and `mission-control`, plus an `openActivitySurface` helper for activity-bar destinations.
- [x] 6.2 Add the matching English and Simplified Chinese entries to `docs/user-guide/screenshots.json`: 48 → 60 entries across 30 scenarios.
- [x] 6.3 Generate the assets and reference them from the chapters with localized alternative text. Six chapters that previously carried no image now carry one.
- [x] 6.4 Developer-guide diagrams: the English guide went from 14 Mermaid diagrams to 42, matching the Chinese guide.
- [x] 6.5 Every new capture comes from the deterministic Web/mock fixture set under a fixed viewport, locale, colour scheme, and clock, with no credential, token, or personal filesystem path in frame.
- [x] 6.6 Add Mission Control coverage to `observability.md` in both guides. The capability was delivered and appeared in neither guide.

## 7. Verification

- [x] 7.1 `npm run docs:check` — passes, including the new reachability gate.
- [x] 7.2 `npm run docs:screenshots:check` — 60 passed, byte-identical to the committed assets.
- [x] 7.3 `npm run docs:build` — assembled both user guides, both developer guides, and the Rustdoc reference; the assembled-mode validation, including reachability, passed.
- [x] 7.4 `npm run lint:ci` — passes.
- [x] 7.5 `openspec validate --specs --strict` — 142 passed, 0 failed.
- [x] 7.6 `openspec validate restore-documentation-spec-conformance --strict` — valid.
- [x] 7.7 The landing page links all four guides plus the Native API Reference, and the relocated Skill Tool runtime security chapter is built in both languages.
