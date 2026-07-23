## Why

VaneHub can correlate sessions, messages, operations, and managed Agent CLI processes, but it cannot reconstruct one causal execution path across user submission, Agent invocation, child-process lifecycle, tool use, and MCP activity. A shared execution-observability model is needed now to make runtime failures, latency, provider differences, and future multi-Agent orchestration diagnosable without relying on unstructured logs or sensitive prompt capture.

## What Changes

- Introduce a per-submission execution run with independent run, trace, and span identities that correlate existing session, message, operation, Agent, provider-session, process, tool-call, and MCP-session identifiers.
- Define a trace topology for task execution, prompt assembly, Agent invocation, managed child-process lifecycle, streamed output, tool execution, MCP requests, cancellation, failure, and completion.
- Classify observed child operations as `native`, `proxied`, `inferred`, or `opaque` so the UI and exported telemetry never imply fidelity that a provider CLI did not expose.
- Add provider-aware normalization for tool lifecycle events and preserve explicit gaps when a CLI reports only a start event or no tool/MCP details.
- Add instrumentation for VaneHub-managed MCP client and connection flows and define an opt-in proxy boundary for high-fidelity observation of MCP traffic launched through VaneHub-managed Agent configurations.
- Persist a bounded, redacted execution timeline in SQLite for local inspection while keeping full prompt, tool arguments, tool results, and model output disabled by default.
- Add optional OTLP export for traces, metrics, and correlated logs without requiring an external Collector for normal desktop use.
- Add desktop telemetry settings for export enablement, OTLP endpoint validation, capture policy, and trace retention while keeping export and content capture disabled by default.
- Extend the frontend service boundary and both Tauri and Web/mock adapters with contract-compatible execution trace queries and deterministic mock timelines.
- Correlate unified diagnostic and operation logs with run, trace, and span identifiers while retaining existing redaction, rotation, archival, and page-level operation output behavior.
- Exclude deterministic task replay, recorded tool-result substitution, and live re-execution from this change; those require a separate execution-journal and replay-safety proposal.

## Capabilities

### New Capabilities

- `agent-execution-observability`: Defines execution run identity, trace topology, fidelity classification, local timeline inspection, optional OTLP export, metrics, privacy defaults, and future multi-Agent correlation semantics.

### Modified Capabilities

- `contract-and-task-foundation`: Extends observable operation contracts with execution-run correlation and hierarchical trace summary fields across Tauri and Web/mock adapters.
- `session-runtime-management`: Requires session message execution, Agent invocation, managed CLI processes, streaming, cancellation, and terminal outcomes to participate in one correlated execution trace.
- `mcp-client-management`: Requires VaneHub-managed MCP activity to emit correlated lifecycle telemetry and defines the managed proxy boundary used when high-fidelity Agent-to-MCP observation is enabled.
- `unified-log-management`: Requires persisted diagnostic and operation logs to carry safe execution correlation fields and supports optional OpenTelemetry log export without bypassing local redaction.
- `app-settings`: Adds desktop-managed observability export, retention, and content-capture settings with Web/mock parity and safe defaults.

## Impact

- Desktop runtime: Rust Agent runtime, process adapters, MCP transport/configuration paths, operations, unified logging, bootstrap, SQLite schema/migrations, and new telemetry infrastructure.
- Frontend and Web runtime: agent/settings service contracts, Tauri adapters, Web/mock adapters, shared types, execution timeline UI, and adapter conformance tests. React remains isolated from Tauri APIs and telemetry storage.
- Dependencies: Rust `tracing` and OpenTelemetry API/SDK/OTLP integration crates, selected as one compatible version family; an external Collector or vendor backend remains optional.
- Data and privacy: new bounded SQLite trace metadata with retention controls; raw prompts, tool arguments/results, credentials, environment values, and model output are not captured by default.
- Compatibility: no intended breaking command or service removal. Existing operation IDs and chat events remain valid and become correlated attributes rather than OpenTelemetry trace identifiers.
