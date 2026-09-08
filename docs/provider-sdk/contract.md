# Provider SDK Contract

The Provider SDK is an internal Rust contract in the `agent_runtime` bounded context. Providers are statically composed into `ProviderRegistry`; application and Session code resolves a stable provider id and negotiates a typed `ProviderCapability` before asking an adapter to act.

Every provider supplies validated metadata, readiness prerequisites, a bounded non-interactive version probe, capabilities, parser and cancellation policies, health classification, permission/model/reasoning option validation, generation translation, interactive translation, resume ownership, and usage/output normalization. Adapter output is expressed through the existing runtime event and error vocabulary. Generic callers must never branch on a provider id or display name.

Capability negotiation is fail-closed. `ProviderRegistry::require` returns `UnsupportedCapability` before launch when a declaration does not support the requested operation. Runtime session references are opaque and provider-owned; a provider rejects a reference belonging to another provider.

Every provider declares the transports it supports: `terminal` (a PTY), `headless` (one process per turn, structured stdout), and `acp-stdio` (a long-lived Agent Client Protocol process over newline-delimited JSON-RPC). The composite process gateway routes a generation request by the resolved provider's transport and refuses a provider that supports neither managed transport before any process starts. Optional capabilities such as usage, resume, reasoning, and model selection may be declared unsupported or unavailable; the mandatory lifecycle responsibilities (launch, prompt delivery, cancellation, terminal classification) remain required for every transport. The ACP transport is described in the developer guide chapter `acp-runtime.md`.

Compatibility policy is additive within schema version 1. Existing built-in executable names, managed arguments, prompt delivery, output events, terminal usage and persisted session identifiers remain stable. Contract changes that cannot preserve those behaviors require a new OpenSpec change and, for manifest changes, a new schema version.

External package discovery and loading are deliberately outside this SDK version.
