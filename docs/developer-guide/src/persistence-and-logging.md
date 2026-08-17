# Persistence and unified logging

## SQLite ownership

SQLite is accessed only from Rust infrastructure. Migrations have a global order, but each schema and repository belongs to a bounded context. A foreign-key reference does not grant one context permission to query another context's tables directly.

Migration changes require:

- a versioned migration;
- clean-database and upgrade-path coverage;
- explicit row-to-domain mapping;
- compatibility with current fixtures;
- no `unwrap()` or `expect()` across production command boundaries.

## Logging

Native diagnostics and operation output flow through the unified logging service. Feature-specific log files are prohibited.

Persisted events must:

- carry `error`, `warn`, `info`, or `debug` semantics;
- redact credentials, tokens, user content, paths, and command-sensitive values before disk writes;
- correlate long-running operations without putting raw prompts or Agent output in diagnostic channels;
- preserve page-visible operation output in its owning result store.

React cannot write local log files. Persisted frontend errors cross the service boundary to the native logging command. Web/mock behavior may expose page-visible simulated logs but cannot claim native persistence.

Execution observability correlation rules are governed by `openspec/specs/agent-execution-observability/spec.md` and `openspec/specs/unified-log-management/spec.md`; the semantic/log-store split is recorded as ADR-002 in `src-tauri/ARCHITECTURE.md`.
