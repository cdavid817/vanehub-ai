## Reuse and ownership map

| Concern | Existing authority reused by Mission Control |
| --- | --- |
| Canonical Run snapshot, event order, cancel/resume | `operations::AgentRunsApi` and its application/domain repository boundary |
| Agent generation and Loop owner behavior | Published `agent_runtime::AgentRuntimeApi` |
| Plan execution and verification | Published `task_orchestration::TaskOrchestrationApi` |
| Evaluation metrics and verification evidence | Existing evaluation application contracts and canonical Run links |
| Execution timeline and tool spans | Published `execution_observability::ExecutionObservabilityApi` |
| Pending approvals | Published `permissions` approval contracts and existing approval UI |
| Review | Existing Code Review Center service and Changes workspace destination |
| Session metadata, usage, logs | Published `sessions` APIs and unified logging query contract |
| Goal/Plan/Loop navigation | Existing stable owner/link identities and workspace destinations |
| Frontend runtime boundary | `AgentService`, `tauri-agent-client`, and `web-agent-client` |
| Navigation and keep-alive | `MainLayout`, workspace route parser, and activity bar |

Mission Control adds a query projection and presentation only. It does not own execution, approval, review, verification, logs, or content evidence, and it does not introduce a bounded context.

## Implementation and verification evidence

- Persistence: no migration was added. The existing additive canonical Run schema and `idx_agent_runs_state` satisfy the audited query plan; migration/reopen/idempotency suites passed unchanged.
- Structural performance: a 125-Run fixture returns at most 20 summaries per section with a cursor, overview uses a fixed counts query plus three bounded section queries, the state query plan uses `idx_agent_runs_state`, detail manifests exclude evidence bodies, and event-coalescer tests bound progress batches while immediately flushing urgent transitions.
- Security and compatibility: projections allowlist safe fields and omit prompts, responses, tool payloads, credentials, raw errors, unrestricted paths, logs, diffs, token/cost guesses, and artifact bodies. Invalid cursor, stale version, unsupported owner/action, terminal idempotency, retry exhaustion, command-safe error, and camelCase serialization coverage passed.
- Frontend: Vitest 279 files / 1278 tests passed. Coverage passed at 70.41% statements, 66.64% branches, 66.15% functions, and 74.43% lines. Contract tests passed 3/3.
- Native: Rust library 3436 passed / 13 ignored, permission-hook 15/15, Architecture Fitness 29/29, MCP contract/integration 6/6, Clippy with `-D warnings`, fmt check, and cargo check passed.
- Browser E2E: Mission Control functional and four visual cases passed 5/5. Repository-wide Playwright passed 146/147 in one loaded run; the sole unrelated initial-workspace timing case passed immediately in isolated retry. The previously affected activity-bar keyboard-order case passed in the full rerun.
- Visual inspection: futuristic/minimal at 1440px and 390px showed readable contrast, no page-level horizontal overflow, no overlap, no blank panel, horizontally bounded summary/detail scrollers, and stable dense layout.
- Desktop: desktop harness unit tests passed 11/11. Linux WebKitGTK native smoke passed through the real Tauri IPC boundary and retained evidence under `test-results/desktop/2026-08-17T08-54-49-529Z-6d4c5017`. Windows and macOS were not run.
- OpenSpec: main specs passed strict validation (134/134) and `add-agent-mission-control` passed strict validation before implementation and again after verification.
