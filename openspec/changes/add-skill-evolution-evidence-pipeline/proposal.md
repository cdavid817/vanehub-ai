## Why

Self-evolution cannot safely generate Skill changes from raw conversations, logs, or anecdotal failures. VaneHub AI needs a privacy-first evidence layer that captures only structured, attributable runtime outcomes, redacts sensitive material before persistence, and produces bounded candidate seeds without modifying any Skill.

## What Changes

- Add six deterministic signal extractors for explicit user feedback, execution/tool failures, verification outcomes, retry/recovery deltas, delegated Utility outcomes, and Skill usage/lifecycle anomalies.
- Capture signals from OnePiece, custom native API Agents, delegated Utility children, managed CLI runs, interactive CLI boundaries, Plan execution, and explicit chat feedback using existing structured runtime events rather than scraping feature log files.
- Record the exact effective Skill ids and revisions observed by native runs; classify CLI attribution as verified, correlated, weak, or unattributed according to available binding, mount, hook, and terminal evidence.
- Prohibit weak or unattributed CLI evidence from becoming a Skill-targeted seed automatically.
- Add a versioned privacy sanitizer with twelve deterministic redaction classes covering credentials, private keys, authorization artifacts, password assignments, credential-bearing URLs and connection strings, secret environment values, user-home paths, email, phone, network identifiers, and account/project identifiers.
- Redact and bound signal summaries before SQLite persistence, hashing, unified diagnostics, query projection, seed construction, or optional telemetry export; raw prompts, transcripts, commands, tool results, files, credentials, and hidden reasoning are not stored in evidence records by default.
- Deduplicate source events idempotently and build deterministic candidate seeds by grouping compatible signals using canonical Skill revision, category, task fingerprint, workspace scope, evidence strength, and time window.
- Persist immutable source references, extractor and sanitizer versions, attribution rationale, polarity, severity, confidence inputs, and evidence lineage so later target selection and review can audit every seed.
- Add bounded retention, per-workspace quotas, corruption-safe ingestion, purge operations, and backpressure that never blocks the originating Agent execution.
- Add explicit helpful/unhelpful and correction feedback on completed assistant messages through the existing frontend service boundary.
- Add read-only per-Skill evolution evidence summaries, signal funnel, attribution distribution, source-Agent distribution, seed inspection, retention status, and purge controls with matching Tauri and Web/mock adapters.
- Keep target Skill selection, quality review, LLM judgment, change generation, Overlay mutation, Curator decisions, scheduling, and automatic application out of this change.

## Capabilities

### New Capabilities

- `skill-evolution-evidence`: Defines evidence sources, six extractors, privacy sanitization, attribution, deduplication, candidate-seed construction, lineage, retention, quotas, queries, purge, and fail-open runtime isolation.

### Modified Capabilities

- `agent-execution-observability`: Adds a bounded local projection of structured execution outcomes and Skill revision associations into the evidence ingestion boundary without weakening telemetry privacy.
- `chat-experience`: Adds explicit completed-message feedback and optional correction capture as a structured evidence source through the frontend service boundary.
- `skill-management`: Adds read-only Skill evidence summaries, bounded signal and seed queries, attribution explanations, retention state, and scoped purge operations.
- `settings-skill-management-ui`: Adds the evidence-only portion of the per-Skill Evolution area, including collection state, signal funnel, attribution, source distribution, seed lineage, privacy status, retention, and purge controls.

## Impact

- Depends on effective Skill revisions and usage tracking from `establish-effective-skill-runtime`; delegated outcomes become richer after `add-delegated-utility-skills` but the pipeline must tolerate that capability being absent.
- Affects execution event projection, Agent runtime and CLI adapters, Plan verification events, chat feedback persistence, new SQLite evidence tables, retention jobs, unified logging, Tauri commands, service contracts, Web/mock behavior, Skills settings, and localization.
- Does not write feature-specific log files, store raw default transcripts, invoke an LLM, mutate Skill or Overlay content, infer unobservable CLI behavior, or let evidence processing delay or fail the originating Agent task.
