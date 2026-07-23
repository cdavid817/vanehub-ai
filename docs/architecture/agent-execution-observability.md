# Agent Execution Observability Compatibility Baseline

This document pins the compatibility baseline for the execution-observability
implementation. The baseline was verified on 2026-07-23 without starting an
interactive Agent session.

## Rust and OpenTelemetry baseline

| Component | Pinned version | Required features |
| --- | --- | --- |
| Rust / Cargo | 1.97.0 / 1.97.0 | `x86_64-pc-windows-msvc` |
| `tracing` | 0.1.41 | attributes and standard library |
| `tracing-subscriber` | 0.3.22 | registry |
| `tracing-opentelemetry` | 0.33.0 | trace bridge |
| `opentelemetry` | 0.32.0 | trace, metrics, logs |
| `opentelemetry_sdk` | 0.32.0 | trace, metrics, logs |
| `opentelemetry-otlp` | 0.32.0 | HTTP/protobuf, blocking HTTP client, trace, metrics, logs |
| `opentelemetry-appender-tracing` | 0.32.0 | tracing-to-OTel log bridge |
| `opentelemetry-semantic-conventions` | 0.32.0 | experimental GenAI constants |

The initial exporter supports OTLP over HTTP/protobuf only. gRPC and alternate
HTTP clients are deliberately excluded until a concrete deployment requires
them. W3C Trace Context propagation is provided by the OpenTelemetry API and
does not require another crate feature.

GenAI and MCP mappings target the OpenTelemetry GenAI semantic-convention
schema `https://opentelemetry.io/schemas/gen-ai/1.42.0`; general resource and
process attributes target semantic conventions 1.43.0. The Rust semantic
conventions crate does not yet expose every MCP/GenAI constant from that schema,
so missing names must be isolated in the telemetry infrastructure adapter and
must include the schema version. Application and domain code must not depend on
OpenTelemetry constants or SDK types.

## Provider capability matrix

The matrix was tested against installed CLI help and version output only.
Availability checks must continue to use non-interactive `--version` and
`--help` probes; they must never start an Agent conversation.

| Provider | Tested CLI | Structured tool lifecycle input | Invocation-scoped managed MCP configuration | Initial observation result |
| --- | --- | --- | --- | --- |
| Claude Code | 2.1.217 | `stream-json` exposes tool-use identity and parent linkage; committed fixtures must prove terminal mappings | Yes, `--mcp-config` accepts a JSON file or value for one invocation | Tool events inferred; managed relay proxied when enabled |
| Codex CLI | 0.145.0 | JSONL events expose stable item/tool identities; committed fixtures must prove start, completion, and failure mappings | Yes, `-c key=value` can override `mcp_servers` for one invocation | Tool events inferred; managed relay proxied when enabled |
| Gemini CLI | 0.51.0 | `--output-format stream-json` is available; committed fixtures must prove lifecycle boundaries | No safe invocation-scoped server-definition option is advertised; allow-list flags do not define a server | Tool events inferred; MCP is opaque unless traffic is VaneHub-native |
| OpenCode | 1.18.4 | `run --format json` is available; committed fixtures must prove lifecycle boundaries | No safe invocation-scoped server-definition option is advertised | Tool events inferred; MCP is opaque unless traffic is VaneHub-native |

Provider output is evidence, not authority to invent missing boundaries. A
start-only call remains incomplete when the Agent exits. Duplicate and
out-of-order events are reconciled by stable provider call id, never display
name. If a provider loses a required call id or the MCP traffic does not cross a
VaneHub-owned boundary, the UI and API must report the fidelity gap rather than
claiming native observation.

## Probe commands

The verified commands were:

```text
rustc --version --verbose
cargo --version
claude --version; claude --help
codex --version; codex --help; codex mcp --help
gemini --version; gemini --help; gemini mcp --help
opencode --version; opencode --help; opencode run --help
```

Capability detection in production may use equivalent read-only probes and
bounded timeouts. It must not run mutating MCP management commands.
