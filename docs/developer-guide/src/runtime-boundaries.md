# Runtime and service boundaries

React components depend on typed frontend services. They must not import Tauri `invoke()` or open SQLite, spawn CLIs, or inspect the local filesystem directly.

## Desktop path

1. A component calls a service interface.
2. The Tauri frontend adapter maps the request to a declared command.
3. A thin Rust command validates and maps transport DTOs.
4. The owning native application service performs the use case through injected ports.
5. Infrastructure adapters perform SQLite, process, filesystem, network, or OS work.

Potentially slow work returns an operation identity before completion and exposes progress through the operations boundary.

## Web/mock path

The Web adapter implements the same frontend contract with deterministic in-memory state. It may simulate execution and timing for UI development, but it must not claim that a local process ran, SQLite changed, or an operating-system action occurred.

## Adding a capability

- Extend the runtime-independent service interface first.
- Implement both the Tauri and Web/mock adapters when the UI consumes the capability.
- Keep provider-specific launch behavior behind Agent Runtime infrastructure.
- Keep user-visible errors localized and native diagnostics in the unified redacted log pipeline.

The architecture rationale and contract examples are expanded in [type contracts](../reference/architecture/type-contracts.md) and [CLI chat runtime v1](../reference/architecture/cli-chat-runtime-v1.md).
