# Unified logging

The single write pipeline for native diagnostic and operational output: level semantics, directory maintenance, redaction before anything reaches disk, and how logs correlate with execution traces by identifier.

SQLite ownership and migrations are in [Persistence ownership](persistence-ownership.md).

## Logging

Native diagnostics and operation output flow through the unified logging service. Feature-specific log files are prohibited.

Persisted events must:

- carry `error`, `warn`, `info`, or `debug` semantics;
- redact credentials, tokens, user content, paths, and command-sensitive values before disk writes;
- correlate long-running operations without putting raw prompts or Agent output in diagnostic channels;
- preserve page-visible operation output in its owning result store.

React cannot write local log files. Persisted frontend errors cross the service boundary to the native logging command. Web/mock behavior may expose page-visible simulated logs but cannot claim native persistence.

Execution observability correlation rules are governed by `openspec/specs/agent-execution-observability/spec.md` and `openspec/specs/unified-log-management/spec.md`; the semantic/log-store split is recorded as ADR-002 in `src-tauri/ARCHITECTURE.md`.

## Unified logging architecture

Native diagnostics and operation output all flow through the unified write pipeline in `src-tauri/src/platform/logging.rs`. A `LogEntry` entering `write_entry` first triggers directory maintenance (rate-limited to once an hour), then redaction, and only then reaches disk. Logging and the execution-observability trace are separated by responsibility — raw prompts and Agent output never enter the diagnostic channel — but run, trace, and span ids are written into the log entry's `context` map as correlation fields, injected by `AgentRuntimeLoggingAdapter::record` and persisted through `UnifiedLoggingAdapter`.

```mermaid
flowchart TD
    Entry(["LogEntry enters write_entry"]) --> Maintain{"maintain_log_dir<br/>less than 1h since last?"}
    Maintain -- yes --> SkipMaint["Skip directory maintenance"]
    Maintain -- no --> Rotate["rotate_active_log<br/>active log mtime > 24h<br/>renamed to vanehub-timestamp.log"]
    Rotate --> Archive["archive_expired_logs_at<br/>files older than 30 days<br/>moved into the archive subdirectory"]
    Archive --> Redact
    SkipMaint --> Redact["redact_entry before the disk write"]
    Redact --> RedactPath["private path → [REDACTED_PATH]"]
    Redact --> RedactBearer["Bearer xxx → Bearer [REDACTED]"]
    Redact --> RedactToken["provider token<br/>sk- / ghp_ prefixes"]
    Redact --> RedactKey["sensitive keys<br/>password / token / secret / credential"]
    RedactPath --> Serialize["serde_json serializes to one line"]
    RedactBearer --> Serialize
    RedactToken --> Serialize
    RedactKey --> Serialize
    Serialize --> Append["Append to the active log file"]
```

Redaction and isolation, in detail:

- **Private paths** are replaced with `[REDACTED_PATH]` before the disk write, so a user's private absolute path never leaves the machine.
- **Bearer tokens** in the shape `Authorization: Bearer xxx` are normalized to `Bearer [REDACTED]`, keeping only the scheme.
- **Provider tokens** are recognized by prefixes such as `sk-` and `ghp_` and erased whole.
- **Sensitive keys** matching names like `password`, `token`, `secret`, and `credential` have their values cleared.
- **Trace correlation** — execution observability's run, trace, and span ids are written into the log entry's `context` field by `AgentRuntimeLoggingAdapter::record` and persisted by `UnifiedLoggingAdapter`. Logs keep only safe metadata: server and language identifiers, lifecycle transitions, method categories, durations, counts, restart attempts, timeout and cancellation categories, exit codes, and safe workspace identifiers. Raw protocol payloads, source code, hover content, diagnostic messages, stderr, environment variables, executable arguments, credentials, and private absolute paths are never persisted.
- **Rate limiting and rotation** — directory maintenance runs at most once an hour; an active log older than 24 hours is renamed and archived; a file in the archive directory older than 30 days moves into the `archive` subdirectory for cold retention.
- **React writes no local logs** — a frontend error that needs persisting crosses the service boundary to the native logging command. Web/mock behavior may expose page-visible simulated logs but cannot claim native persistence.

## Logging constants and redaction rules

### Logging constants

| Constant | Value | Meaning |
| --- | --- | --- |
| `LOG_FILE_NAME` | `"vanehub.log"` | The active log file name |
| `ARCHIVE_DIR_NAME` | `"archive"` | The cold-retention subdirectory name |
| `RETENTION_DAYS` | `30` | Files older than this move into the `archive` subdirectory |
| `ROTATION_AGE_HOURS` | `24` | An active log with an mtime older than this is renamed and archived |
| `MAINTENANCE_INTERVAL_HOURS` | `1` | Directory maintenance runs at most this often |

### Logging types

The `LogLevel` enum carries four values: `Error`, `Warn`, `Info`, and `Debug`. A `LogEntry` has the fields `timestamp`, `level`, `category`, `message`, and `context`. `ClientLogEvent` carries events reported by the frontend across the service boundary, such as `ErrorBoundary` and `CriticalOperationFailure`.

### Redaction

`redact_text` and `redact_entry` redact once before the disk write and once before JSON serialization, covering four classes:

- **Private paths** → `[REDACTED_PATH]`, matching absolute-path prefixes such as `C:\`, `/home/`, `/Users/`, and `file:///`.
- **Bearer** → `Bearer [REDACTED]`, keeping only the scheme.
- **Provider tokens** recognized by prefixes such as `sk-`, `ghp_`, `github_pat_`, and `ssh-connection`, erased whole.
- **Sensitive keys** matching names such as `password`, `token`, `secret`, `credential`, `authorization`, `key_path`, and `private_key`, with their values cleared.

### Trace correlation

Execution observability is carried by `contexts/operations/domain/operation.rs`: every operation has a `trace_id`, and `correlate_execution(run_id, trace_id)` ties a run to a trace. Run, trace, and span ids are written into the log file's `context` field by `AgentRuntimeLoggingAdapter::record`, while raw prompts and Agent output stay out of the diagnostic channel.

The source of truth for the unified logging specification is `openspec/specs/unified-log-management/spec.md`; execution observability correlation rules live in `openspec/specs/agent-execution-observability/spec.md`.
