## Why

OnePiece can now measure, optimize, suppress, and explain context compaction, but operators still cannot tell whether the policy remains effective across real sessions or detect regressions before changing thresholds. A privacy-safe quality ledger, deterministic evaluation corpus, and bounded health summary are needed before provider-specific cache edits or more aggressive policy tuning are introduced.

## What Changes

- Add a content-free context quality assessment for every automatic compaction attempt, including success, bypass, fallback, and failure outcomes.
- Persist bounded assessment records in SQLite with generation correlation, policy versions, measurement quality, structural-retention checks, savings, path, reason codes, and timestamps, without raw prompts, summaries, tool payloads, or secrets.
- Add deterministic regression fixtures and a policy evaluator that compares candidate policies against the active baseline without changing live generation behavior.
- Expose typed desktop and Web/mock service contracts for bounded history and aggregate health summaries.
- Add a localized OnePiece context-health surface showing compaction rate, savings, quality coverage, fallback/failure distribution, and policy version, with explicit non-billing and non-semantic-quality disclosures.
- Correct the native-agent architecture documentation so it reflects the delivered evidence UI and persisted automatic-compaction control.
- Keep manual “compact now”, provider-native prompt-cache edits, automatic policy rollout, evidence export, and model-judge scoring out of scope.

## Capabilities

### New Capabilities

- `agent-context-quality-evaluation`: Content-free compaction outcome assessment, bounded persistence, deterministic regression evaluation, baseline comparison, and policy-health aggregation.

### Modified Capabilities

- `agent-context-evidence-projection`: Correlate each successful evidence card with its persisted quality assessment without exposing content.
- `app-settings`: Persist the local retention window used for bounded context-quality history.
- `settings-cli-management-ui`: Display localized OnePiece context-health history and aggregate policy diagnostics through the settings service boundary.
- `frontend-runtime-architecture`: Require compatible desktop and Web/mock contracts for context-quality history and summaries.

## Impact

- **Desktop runtime:** Adds domain assessment types, SQLite schema/migration and repository queries, generation correlation, bounded aggregation, and Tauri commands.
- **Web runtime:** Adds deterministic in-memory assessment records and contract-compatible history/summary responses without network access.
- **Frontend:** Adds typed service operations and a OnePiece settings health panel; React components continue to avoid direct Tauri invocation.
- **Privacy and logging:** Records allowlisted metadata only and routes diagnostics through unified logging; no prompt or provider payload content is persisted.
- **Dependencies:** Uses existing React, Tauri, Rust, SQLite, Vitest, and Playwright foundations; no new third-party dependency is introduced.
