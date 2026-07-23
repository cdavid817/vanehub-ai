## Context

VaneHub already creates durable sessions and messages, an in-memory observable operation, a managed Agent CLI child process, normalized streaming events, and redacted unified logs. Those records share identifiers informally, but there is no causal execution model, parent/child relationship, duration model, propagation mechanism, or queryable cross-layer timeline. Tool-use records are reconstructed from provider output and may contain only a start event. The native MCP implementation currently manages server configuration and connection tests; Agent CLIs may execute tools or connect to MCP servers internally without passing traffic through VaneHub.

The change crosses the Agent runtime, operation model, process layer, MCP transports, logging, settings, SQLite, Tauri commands, frontend adapters, and UI. It must preserve desktop/Web contract parity, avoid direct Tauri calls from React, work without an external observability backend, and never weaken the existing redaction rules.

OpenTelemetry GenAI and MCP semantic conventions are still evolving. The implementation therefore needs a pinned schema version and a small `vanehub.*` namespace for concepts such as user task runs and observation fidelity that do not yet have stable standard attributes.

## Goals / Non-Goals

**Goals:**

- Correlate a submitted user task with Agent invocation, managed child-process lifecycle, normalized tool events, VaneHub-managed MCP activity, streaming milestones, cancellation, and terminal outcome.
- Provide trustworthy topology by identifying every child observation as native, proxied, inferred, or opaque.
- Persist a bounded metadata timeline locally and expose it through the same frontend service and runtime-adapter pattern used elsewhere.
- Export correlated traces, metrics, and logs over OTLP when explicitly enabled.
- Preserve unified local logs, page-level operation logs, existing chat events, and current operation identifiers.
- Establish parent Agent and span-link semantics that future multi-Agent orchestration can use without redesigning trace identity.
- Keep sensitive content collection opt-in, bounded, redacted, and separate from trace correlation.

**Non-Goals:**

- Deterministic task replay, substitution of recorded tool results, or live re-execution.
- Capturing provider chain-of-thought, credentials, raw environment variables, unrestricted prompts, or unrestricted tool payloads.
- Making externally configured Agent CLI MCP servers observable when their traffic does not cross a VaneHub-managed boundary.
- Replacing the existing unified log directory with an OpenTelemetry backend.
- Requiring a Collector, cloud account, or network connection for normal desktop operation.
- Instrumenting provider model-service HTTP traffic hidden inside third-party Agent CLI binaries.

## Decisions

### 1. Use an execution run as the product-level correlation root

Every accepted desktop, IM, or scheduled message submission creates an `ExecutionRun` before prompt assembly. It has a generated `run_id`, W3C-compatible `trace_id`, root `span_id`, source, lifecycle, timestamps, and safe links to the existing session, user message, assistant message, operation, and stable Agent ids.

`operation_id` remains an application operation identity and is not reused as a trace id. Provider-native session ids remain resume metadata and are attributes, not trace identities. A future retry creates a new run and trace, then links to the prior run instead of reusing its trace id.

Rationale: session ids are conversation-scoped, message ids are persistence-scoped, and operation ids have an existing public shape. A separate run identity gives each submission an immutable correlation root without breaking current contracts.

Alternative considered: use `operation_id` as `trace_id`. Rejected because it is not W3C-shaped, operations are currently in-memory, and conflating product and telemetry identity would make migration and external export brittle.

### 2. Define an explicit trace topology and carry context across background boundaries

The initial topology is:

```text
vanehub.task.execute
  |-- vaneHub.prompt.assemble
  `-- invoke_agent <stable-agent-id>
        |-- vaneHub.process.run <stable-agent-id>
        |-- execute_tool <tool-name>
        |     `-- MCP client/server spans when traffic is managed
        `-- invoke_agent <child-agent-id> or a linked child run
```

The root uses a custom `vanehub.task.execute` internal span until task-level GenAI conventions stabilize. Agent and tool spans follow the pinned OpenTelemetry GenAI conventions. VaneHub-managed MCP spans follow the pinned MCP conventions. Custom attributes use `vanehub.*`; standard attributes are not duplicated under custom names.

The application model carries an opaque `ExecutionContext` containing correlation identity across `GenerationProcessRequest`, generation coordination, monitoring threads, event sinks, MCP relay work, and cancellation. Infrastructure converts it to OpenTelemetry context. Thread-local span state alone is insufficient because current generation monitoring crosses synchronous calls and native threads.

### 3. Put telemetry behind native application ports

Agent, operation, and MCP application services depend on an `ExecutionTelemetryPort`, not OpenTelemetry SDK types. The port supports run start/finish, scoped stage start/finish, events, links, and metrics using bounded domain records. The Rust infrastructure implementation composes:

- a `tracing` subscriber and OpenTelemetry SDK/export layer;
- a SQLite execution timeline repository;
- the existing unified logging adapter for correlated diagnostics.

The OpenTelemetry resource identifies the desktop application, version, runtime environment, and a generated installation instance id. It does not include username, project path, prompt text, or credential material.

Rationale: application tests can capture deterministic telemetry records, OpenTelemetry dependencies stay in infrastructure/bootstrap, and the domain does not become coupled to an evolving SDK.

Alternative considered: call OpenTelemetry directly from each service and process adapter. Rejected because it would spread context management, privacy decisions, and exporter failure behavior across feature modules.

### 4. Retain a normalized local timeline independent of OTLP

SQLite stores bounded `execution_runs` and `execution_spans` records plus bounded lifecycle events required by the local viewer. Records contain identifiers, span names/kinds, start/end timestamps, status, safe low-cardinality attributes, fidelity, and error classification. Large payloads and raw stdout/stderr are not stored in these tables.

Local trace retention defaults to 30 days and is configurable from 1 to 90 days. Cleanup runs on a bounded maintenance schedule rather than per event. The existing unified log files retain their current rotation and archival policy independently.

Rationale: external OTLP export may be disabled or offline, while desktop debugging still needs a queryable timeline. A normalized product view is also stable across exporter or semantic-convention upgrades.

Alternative considered: read JSONL unified logs to build the timeline. Rejected because log rotation, archival, free-form messages, and missing parent relationships do not provide reliable queries.

### 5. Export asynchronously and remain local-first

OTLP export is disabled by default. When enabled, the desktop settings store a validated endpoint, protocol selection, sampling ratio, and metadata capture policy. Optional authentication material is stored through the native credential store, not SQLite. Web/mock mode keeps deterministic settings and timelines but does not claim native export.

The Rust SDK uses bounded batch processors and short export timeouts. Exporter failure never fails a user task; it emits a rate-limited, redacted local diagnostic without recursively exporting that diagnostic through the failing path. Shutdown attempts a bounded flush.

Local metadata timelines record every accepted run. External sampling applies only to OTLP export. Metrics exclude run, session, message, operation, process, and tool-call ids as dimensions.

### 6. Treat content as opt-in data, not correlation context

The default `metadata_only` policy records names, statuses, durations, counts, versions, safe error classifications, and hashes where useful. It does not record prompt text, model output, tool arguments/results, command-line prompt arguments, full paths, headers, environment values, or MCP payload bodies.

An explicit `redacted_content` policy may include bounded, redacted summaries of tool arguments/results and provider event metadata. Redaction occurs before SQLite persistence, unified logging, and OTLP export. Sensitive values are never placed in W3C baggage or process-wide resource attributes.

This change does not introduce raw-content capture. A future replay proposal must define encryption, consent, side-effect classification, and deletion semantics separately.

### 7. Normalize provider tool events without inventing missing facts

Provider adapters map native CLI output into a lifecycle-aware tool event containing call id, name, phase, timestamp when supplied, status, and safe metadata. Events with the same stable call id update one tool span. If a provider reports only a start event, the span ends with an `incomplete` observation state when the Agent process terminates. It is not marked successful merely because the parent process succeeded.

Each observed span includes `vanehub.observation.fidelity` with one of:

- `native`: VaneHub executed the operation and observed both boundaries;
- `proxied`: traffic crossed a VaneHub relay;
- `inferred`: reconstructed from provider CLI events;
- `opaque`: the stage is known to exist but details are unavailable.

Provider capability tests use committed fixtures for Claude Code, Codex CLI, Gemini CLI, and OpenCode, including start-only, completed, failed, duplicated, and out-of-order events.

### 8. Add an opt-in VaneHub-managed MCP relay for high-fidelity spans

Existing VaneHub MCP connection tests are instrumented natively. For Agent execution, high-fidelity MCP tracing is available only when the Agent uses a VaneHub-managed MCP configuration and the provider adapter supports invocation-scoped configuration injection.

The relay presents the provider-compatible stdio or HTTP endpoint, forwards JSON-RPC to the configured MCP server, and records request lifecycle metadata without payload bodies under `metadata_only`. Each invocation receives a run-scoped relay identity so traffic can be correlated even when the third-party CLI does not propagate W3C headers. HTTP trace context is propagated when supported; stdio correlation uses the invocation-scoped relay context. Relay spans are marked `proxied`.

The relay does not silently rewrite global provider configuration. Unsupported provider/configuration combinations continue without the relay and are marked inferred or opaque. Availability checks remain separate and do not launch an interactive Agent session.

Alternative considered: require every Agent CLI or MCP server to adopt VaneHub environment variables. Rejected because static process environment cannot represent individual concurrent tool calls and third-party binaries may ignore it.

### 9. Expose one contract through desktop and Web adapters

A frontend `ExecutionObservabilityService` exposes settings, run summaries, a paginated trace timeline, and run detail. React components call this interface. The Tauri adapter invokes Rust commands; the Web/mock adapter returns deterministic contract-compatible traces and never imports Tauri APIs.

The viewer presents topology, durations, status, fidelity, and safe correlation ids. It does not read SQLite, unified log files, or OTLP endpoints directly. Links from a trace to session and operation views use stable ids already returned through service contracts.

### 10. Keep logs, traces, and metrics correlated but purpose-specific

Unified diagnostic and operation log records gain optional `runId`, `traceId`, and `spanId` context. Existing `operationId`, session, and Agent context remain. The local log remains the durable diagnostic sink and applies redaction before persistence.

Initial metrics cover run and Agent duration, process start/exit failures, cancellation count, tool duration where both boundaries are known, MCP duration, and streaming time to first output. Metric dimensions are limited to stable Agent/provider ids, source, outcome, tool/MCP operation class, and fidelity.

## Risks / Trade-offs

- [Provider event formats are unstable or incomplete] -> Keep provider-specific adapters and fixtures, mark fidelity explicitly, and close unmatched spans as incomplete rather than successful.
- [MCP relay changes execution behavior] -> Make it opt-in, invocation-scoped, restricted to VaneHub-managed configurations, and covered by transparent forwarding and timeout tests.
- [Telemetry captures secrets or source content] -> Default to metadata only, centralize redaction before every sink, bound values, prohibit baggage content, and add leak-oriented tests.
- [OTLP export slows or breaks user tasks] -> Use bounded asynchronous queues, short timeouts, drop accounting, rate-limited local diagnostics, and never return exporter errors from task execution.
- [SQLite timeline grows indefinitely] -> Enforce bounded attributes/events, 1-90 day retention, indexed pagination, and scheduled cleanup.
- [High-cardinality identifiers overload metrics] -> Keep identifiers on traces/logs only and test the allowed metric attribute set.
- [Local and exported views disagree because of sampling] -> Persist every local metadata run and expose export state separately; sampling affects only OTLP.
- [Semantic conventions change] -> Pin one compatible schema/crate family, store schema version on exported resources, isolate mappings in infrastructure, and use `vanehub.*` only for missing concepts.
- [Parallel and retried Agent work cannot be represented by a strict tree] -> Support span links and explicit `parent_agent_run_id`, `delegation_id`, and `attempt` fields without requiring multi-Agent orchestration in this change.

## Migration Plan

1. Add shared execution contracts and additive SQLite migrations with telemetry disabled; existing session and operation behavior remains unchanged.
2. Introduce the native telemetry port, local timeline repository, and unified-log correlation behind a disabled feature setting.
3. Instrument task, Agent, process, stream, cancellation, and terminal boundaries and enable the local metadata timeline by default.
4. Add provider tool lifecycle normalization and fidelity reporting, then enable provider adapters one at a time behind capability flags.
5. Instrument existing MCP connection flows and ship the managed relay disabled by default until provider-specific forwarding tests pass.
6. Add service adapters, settings, and the local timeline UI; keep Web/mock behavior deterministic.
7. Add optional OTLP export and validate against a local Collector before documenting external backend configuration.

Rollback disables OTLP and MCP relay settings first. Additive SQLite tables and correlation fields can remain unused. Removing the new UI/service registrations restores previous behavior without rewriting sessions, messages, operations, or unified logs.

## Open Questions

- Which provider CLI versions support invocation-scoped MCP configuration without mutating user-global files? The implementation must record a verified capability matrix before enabling relay support for each provider.
- Should the first UI expose trace search across all sources, or only session-scoped timelines? The service contract supports both; tasks will start with session/run lookup unless usability testing justifies global search.
- Which pinned OpenTelemetry GenAI/MCP schema version matches the selected Rust crate family at implementation time? Resolve and record this in dependency verification before adding instrumentation constants.
