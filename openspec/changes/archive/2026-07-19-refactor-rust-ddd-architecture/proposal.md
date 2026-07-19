## Why

The Rust desktop runtime has grown into a 10,000-line `lib.rs` plus feature modules that mix domain rules, application orchestration, SQLite access, external-process integration, Tauri commands, and startup wiring. This makes ownership and dependency direction unclear, increases regression risk across unrelated features, and leaves no enforceable project standard for extending the native runtime consistently.

## What Changes

- Refactor the Rust desktop runtime as a domain-oriented modular monolith with explicit bounded contexts and context-local `domain`, `application`, `infrastructure`, and `interfaces` boundaries.
- Reduce `src-tauri/src/lib.rs` to native bootstrap and composition-root responsibilities; move business rules, use cases, persistence, process/network adapters, and Tauri command handlers into their owning contexts.
- Introduce inward dependency rules: domain code remains independent of Tauri, SQLite, filesystem, network, process, and logging frameworks; application use cases depend on domain types and explicit ports; infrastructure and Tauri adapters implement those ports at the edges.
- Separate domain models from Tauri request/response DTOs and SQLite row representations, and centralize command-safe error mapping at the interface boundary.
- Migrate incrementally by bounded context, preserving existing Tauri command names, serialized contracts, SQLite data/migrations, unified logging behavior, task observability, and frontend service boundaries throughout the refactor.
- Add Rust DDD rules, target layout, dependency constraints, testing expectations, and review guardrails to `openspec/project.md` so future native work follows the same architecture.
- Add automated architecture checks and focused domain/application tests so dependency violations and behavior regressions fail verification.
- This change affects the Tauri desktop runtime only. The React application, Web/mock runtime, and frontend adapter interfaces remain behaviorally unchanged.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `native-runtime-architecture`: Add mandatory bounded-context ownership, layered dependency direction, port/adapter isolation, composition-root, migration-compatibility, and architecture-verification requirements for the Rust native runtime.

## Impact

- Primary code impact: `src-tauri/src/lib.rs`, existing native feature modules, command registration/bootstrap, SQLite repositories and migrations, process/network/filesystem adapters, and native tests.
- Standards impact: `openspec/project.md` gains the authoritative Rust DDD conventions used for all new native features and incremental migration of existing code.
- API impact: no intended breaking changes to Tauri command names, request/response serialization, frontend service interfaces, or Web/mock adapters.
- Data impact: no destructive schema changes; existing app-owned SQLite databases and migration history must remain compatible.
- Logging and operations impact: existing unified logging, redaction, and backend-managed task requirements remain mandatory and move behind explicit application ports/adapters where appropriate.
- Dependency impact: no dependency-injection framework or alternate persistence stack is planned; the initial target remains one Rust crate with module visibility and tests enforcing boundaries.
