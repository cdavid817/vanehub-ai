## Why

VaneHub currently treats persisted `starting` or `running` sessions as failed after restart and marks unfinished assistant messages failed without correlating the generation, message, operation, or possible tool activity that produced them. This repairs visible lifecycle flags but cannot distinguish a safely interrupted generation from conflicting or side-effect-bearing work, so recovery needs durable execution identity, deterministic evidence reconciliation, and an explicit safety gate before broader crash-recovery capabilities can be added.

## What Changes

- Add an orthogonal session recovery status (`clean`, `reconciling`, `action_required`, or `quarantined`) instead of overloading the existing lifecycle state with recovery safety.
- Reuse the existing execution run identifier as the stable identity for every accepted managed generation and persist its association with the owning session and messages.
- Add deterministic per-session message sequencing and session state/history revisions so recovery, pagination, and later history projections do not infer order from timestamps.
- Claim active generations with a durable compare-and-set transition and reject new work while a session is archived, being reconciled, awaiting recovery action, quarantined, or already owns an active run.
- Replace unconditional orphan-to-failed startup handling with an idempotent startup coordinator that reconciles business evidence for the same execution run, preserves partial content, records a recovery report, and surfaces ambiguity rather than guessing.
- Add explicit user acknowledgement for an ambiguous interrupted generation. Acknowledgement clears the recovery gate without deleting evidence, retrying the generation, or claiming that an opaque side effect did not occur.
- Make desktop, headless native callers, Plan, Loop, multi-seat handoffs, and Web/mock presentation consume the same recovery status and service boundary.
- Separate one-shot startup recovery from recurring inactive-session archival and emit typed, revisioned session invalidation events after durable state changes.
- Keep recovery diagnostics in the unified logging service with safe reason codes and correlation identifiers; prompts, message bodies, tool payloads, commands, credentials, and raw provider errors remain excluded.

## Capabilities

### New Capabilities

- `session-recovery`: Defines durable generation correlation, recovery safety states, evidence-based startup reconciliation, recovery reports, acknowledgement, idempotency, and runtime-neutral presentation.

### Modified Capabilities

- `session-management`: Extends durable session/message contracts with recovery state, execution correlation, deterministic ordering, revisions, and evidence-based startup recovery.
- `session-runtime-management`: Replaces process-local generation ownership and unconditional orphan failure with durable claims, correlated terminal persistence, and recovery-aware CLI/API session behavior.
- `chat-experience`: Gates send/stop controls on recovery safety and presents reconciling, action-required, and quarantined states through the frontend service boundary.
- `native-runtime-architecture`: Separates startup reconciliation from recurring maintenance and defines recovery bootstrap ordering, durable transactions, and failure-injection coverage.
- `plan-execution-runtime`: Makes Plan recovery consume shared session terminal evidence instead of independently inferring outcomes from recent messages.
- `loop-engineering-runtime`: Makes Loop recovery consume the same session recovery projection while preserving Loop-owned session isolation and explicit recovery-required outcomes.

## Impact

- **Desktop runtime:** Additive SQLite migrations, session/message repository changes, durable generation-claim and terminal transactions, a startup recovery coordinator, recovery reports, typed session events, and recovery-aware Plan/Loop integration.
- **Web runtime:** Matching normalized recovery fields, service methods, events, and deterministic mock states without claiming native process or SQLite recovery.
- **Frontend:** Shared service types and adapter implementations gain recovery summaries and acknowledgement; React continues to use `agent-service.ts` and does not invoke Tauri directly.
- **Runtime adapters:** API Agents and managed CLI chat share the session-level recovery contract. VaneHub-controlled API execution can later provide stronger tool evidence, while incomplete provider/CLI-internal tool activity remains conservatively ambiguous.
- **Persistence and compatibility:** Existing messages receive deterministic sequence values; historical execution-run associations remain nullable rather than being fabricated. Migrations are additive and retain existing sessions, messages, provider resume metadata, usage, Plan/Loop records, and logs.
- **Out of scope:** Durable tool-effect journaling, automatic tool replay, context budgeting/compaction, payload reduction, checkpoints, session forking, in-place rollback, and LLM-based recovery decisions.
