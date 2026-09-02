# Execution observability

How an execution trace is recorded and layered, which nodes carry visible fidelity, and where a trace stops at a boundary.

The Agent evaluation arena runs on the same Operation lifecycle but is a separate problem domain; see [Evaluation runtime](evaluation-runtime.md).

## Execution traces

### Four core types

| Type | What it is |
| --- | --- |
| `ExecutionRun` | One observable execution, holding a trace id and status |
| `ExecutionSpan` | A segment within a run, name capped at 128 characters |
| `ExecutionEvent` | A point-in-time event on a span |
| `ExecutionTimeline` | The timeline view the UI expands |

`ExecutionStatus` has six states: `Accepted`, `Running`, and four terminal states — `Succeeded`, `Failed`, `Cancelled`, `Incomplete` — of which `is_terminal()` recognizes the latter four.

**`Incomplete` is a terminal state, not an intermediate one.** It means the execution ended but the trace was not fully recorded — distinct from "failed": failure is a conclusion about the execution, incompleteness is a conclusion about the observation.

### Fidelity: the trace declares how much it knows about itself

`ExecutionFidelity` has four tiers, and it's the most important design in this context:

| Fidelity | Meaning |
| --- | --- |
| `Native` | A first-hand record the runtime produced itself |
| `Proxied` | Observed through a relay |
| `Inferred` | Inferred from whatever signals were available |
| `Opaque` | What happened in this segment cannot be known |

**Why `Opaque` has to exist**: an external CLI Agent is a black box — VaneHub starts the process and captures its output, but cannot see its internal tool calls. Drawing this as a span tree that looks complete would let a reader assume they're seeing everything. Declaring `Opaque` says "there really is a segment here, but its contents are unknown" — which is more honest than fabricating a node, and more useful than drawing nothing at all.

OnePiece runs over the native API, so its tool calls carry `Native` fidelity and can be expanded layer by layer; this is exactly the observability advantage the [native Agent](onepiece-native-agent.md) has over an external CLI.

### Capture policy and sanitization

`CapturePolicy` has only two tiers: `MetadataOnly` and `RedactedContent`. **There is no "raw content" tier at all** — even at the most detailed capture setting, content is sanitized.

Attributes carry hard ceilings; going over rejects rather than truncates:

| Limit | Value |
| --- | --- |
| Attributes per set | **32** |
| Attribute key length | **128** characters |
| Attribute value length | **256** characters |

The type is `SafeAttributes` / `SafeAttributeValue` — **"safe" is written into the type name itself**: validation happens at construction, not as a sanitization pass before writing to disk. Trying to stuff arbitrarily long text into the trace fails to compile.

### Execution source

`ExecutionSource` distinguishes three originators: `Desktop`, `InstantMessage { connector_id }`, `Scheduled { task_id }`. IM and scheduled tasks carry their own identifiers, so "who triggered this execution" is first-class information in the trace, not something guessed from a timestamp.

## Relationship to other contexts

- Unified logging and redaction rules are in [Unified logging](unified-logging.md). **Traces and logs correlate on `runId`, `traceId`, and `spanId`** — `AgentRuntimeLoggingAdapter::record` writes all three into a log entry's `context`. Only an external CLI's internal behaviour, an older record, or a degraded path carrying no context needs the fallback to time alignment.
- The Operation lifecycle is owned by `operations`.
- The user-facing surface is in the user guide's observability chapter.

## Where the design lives

This chapter orients contributors; the authoritative requirements live in the capability's specification under `openspec/specs`.
