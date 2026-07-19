## Context

The native crate currently implements more than 100 Tauri commands across agent execution, sessions, projects, CLI lifecycle, SDK/MCP management, extensions, skills, Prompt Hooks, IM connectors, settings, desktop lifecycle, shell access, logging, and observable tasks. Although several newer features already use `models`, `service`, and `commands` modules, dependency ownership is inconsistent: most modules import root-level `AppError` and `RegistryStore`, infrastructure concerns are called directly from services, and `src-tauri/src/lib.rs` still contains 10,178 lines spanning every architectural role.

This is a structural refactor of the Tauri desktop runtime. React continues to call frontend service interfaces; only Tauri-specific frontend adapters call `invoke()`; the Web/mock adapter remains unchanged. Existing Tauri command names, serialized payloads, SQLite contents, operation/log semantics, and user-visible behavior are compatibility constraints.

The desired outcome is a DDD-oriented modular monolith, not a distributed system and not a literal translation of every DDD pattern. Bounded contexts express business ownership. Layers and ports enforce dependency direction only where code has domain behavior or external I/O. Small pure helpers may remain simple modules when an entity, aggregate, repository, or domain service would add no useful boundary.

## Goals / Non-Goals

**Goals:**

- Establish and document a bounded-context map for all native responsibilities.
- Make domain rules testable without Tauri, SQLite, the filesystem, the network, external processes, or OS credential stores.
- Put application orchestration behind explicit use cases and context-specific input/output ports.
- Keep Tauri commands as thin interface adapters and centralize concrete construction in a native composition root.
- Replace root-level database-centric coupling with context-owned repositories and explicit cross-context application contracts.
- Migrate context by context while retaining command, data, logging, task, and frontend adapter compatibility.
- Enforce the dependency rules through Rust visibility plus an automated architecture test included in `cargo test`.
- Add the resulting DDD rules to `openspec/project.md` as mandatory standards for new and migrated Rust code.

**Non-Goals:**

- Changing React screens, frontend service interfaces, Tauri command names, or the Web/mock runtime.
- Rewriting product behavior, redesigning the SQLite schema, or replacing the unified logging/task systems.
- Splitting the native runtime into microservices or introducing messaging infrastructure.
- Splitting every bounded context into a separate Cargo crate during the initial refactor.
- Adding a dependency-injection framework, ORM, generic repository framework, or event-sourcing system.
- Forcing DDD terminology onto stateless platform helpers that contain no domain policy.

## Decisions

### 1. Use a single-crate, domain-oriented modular monolith

The native runtime will remain one Cargo crate. Domain-bearing code will be organized by bounded context, with each context exposing only an explicit application API. The initial context map is:

| Context | Primary ownership |
| --- | --- |
| `agent_runtime` | Agent registry, interaction modes, provider invocation, workflow state, generation lifecycle |
| `sessions` | Sessions, messages, categories, chat configuration, export, maintenance, usage records/read models |
| `workspaces` | Local/remote projects, worktrees, bounded file/Git inspection, session shell lifecycle |
| `tooling` | CLI lifecycle plus the existing SDK, MCP, extension, plugin, Skill, and Prompt Hook subdomains |
| `communications` | IM connector configuration, credentials, protocol adapters, routing, delivery lifecycle |
| `desktop` | App settings, startup, data/log directory actions, floating assistant, window/tray lifecycle |
| `operations` | Observable task lifecycle and unified diagnostic/operation logging contracts |

`tooling` is a parent context with named subdomains, not one shared model. A subdomain may be promoted to a peer context if its language, lifecycle, or transaction ownership proves independent during migration. `usage` starts as a sessions-owned reporting model because its records are owned by assistant messages; this can be revisited by an ADR without changing the dependency rules.

Alternative considered: top-level technical folders such as `models`, `services`, and `repositories`. That arrangement makes shared technology visible but scatters one business capability across the crate and repeats the coupling already present. Separate Cargo crates would enforce boundaries more strongly, but introducing them while behavior is still being extracted would increase build and migration complexity.

### 2. Use context-local layers with an outer Tauri command adapter

Each bounded context will use the following conceptual structure. Empty layers are not created until needed.

```text
src-tauri/src/
  lib.rs
  bootstrap/
    mod.rs
    app_state.rs
    command_registry.rs
  contexts/
    <context>/
      domain/
      application/
        ports/
      infrastructure/
      api.rs
  commands/
    <context>/
      <one-command-per-file>.rs
  platform/
    database/
    process/
    filesystem/
    network/
    credentials/
```

The root `commands/` directory remains the Tauri interface layer and keeps the existing one-command-per-file convention. Handlers may depend on a context's public application API and command DTO mappers, but not on repositories or other infrastructure. Non-Tauri inbound adapters, such as IM transports, live at the outer edge of their owning context.

Alternative considered: placing `interfaces/tauri` inside each context. That offers stronger physical locality, but it conflicts with the project's established command layout and makes global Tauri registration harder to audit. The selected layout preserves domain ownership through dependency rules while keeping native entry points discoverable.

### 3. Enforce inward-only dependencies

The allowed direction is:

```text
commands / external adapters -> application -> domain
infrastructure ---------------> application ports + domain
bootstrap --------------------> all outer implementations for construction only
```

- `domain` contains entities, value objects, aggregate rules, domain services, domain events, and typed domain errors. It cannot import Tauri, Rusqlite, filesystem/process/network APIs, credential stores, logging implementations, infrastructure, commands, or another context's internals.
- `application` contains use cases, transaction orchestration, application errors, input/output models, and narrow ports. It depends on its context's domain and deliberately published cross-context APIs, never concrete infrastructure or Tauri.
- `infrastructure` implements application ports for SQLite, external commands, filesystem, HTTP, credentials, clocks, ids, logging, and task publication. It does not define business invariants.
- `commands` validates and maps transport DTOs, obtains an assembled use case from Tauri state, invokes it, maps output/errors, and emits interface events when required. It contains no SQL, process construction, or domain decisions.
- `bootstrap` is the only module that selects concrete implementations and wires them into `NativeAppState`. Tauri-managed state stores assembled services; domain and application code do not use it as a service locator.

A minimal shared kernel may contain stable identifiers and value types used by multiple contexts. New shared-kernel items require documented consumers and invariant ownership. General utilities and I/O implementations belong to `platform`, not the shared domain model.

Alternative considered: allowing application services to call concrete SQLite/process helpers for speed. That keeps fewer types initially but prevents deterministic tests and recreates root-level coupling.

### 4. Use explicit, behavior-oriented ports and context APIs

Repository and gateway traits will be defined by the consuming application layer and named for required behavior, for example `SessionRepository`, `AgentProcessGateway`, or `OperationPublisher`. They will not expose generic CRUD, raw SQL rows, `rusqlite::Connection`, `tauri::AppHandle`, or untyped maps across the boundary.

Cross-context calls use a small published `api` facade, immutable query DTO, or explicit domain/application event. A context may not import another context's repositories, infrastructure, private aggregates, or command DTOs. Synchronous application APIs remain the default; domain events are introduced only when one completed action has independent downstream reactions.

Alternative considered: a global `RegistryStore` and shared service container. It is convenient but makes every feature depend on a database-shaped abstraction and obscures transaction and ownership boundaries.

### 5. Separate domain, persistence, and transport models

Tauri request/response DTOs retain their current serialized shapes and live at the interface boundary or in an explicitly shared contract module. SQLite row structs and conversion code live in infrastructure. Domain constructors and value objects enforce business invariants; invalid states cannot be introduced merely by deserializing a command payload or reading a row.

Conversions are explicit and fallible where data can be invalid:

```text
Tauri DTO -> validated application input -> domain type
SQLite row -> infrastructure mapper -> domain type
domain/application output -> Tauri response DTO
```

Serde derives are allowed on domain types only when serialization is itself part of the domain contract. Database column names and frontend compatibility aliases do not leak into domain types.

Alternative considered: reusing one struct for commands, domain logic, and database rows. It reduces mapping code but couples invariants to both storage and transport evolution.

### 6. Put transaction and side-effect sequencing in application use cases

Each modifying use case defines its consistency boundary. Context-specific persistence ports expose atomic operations or a transaction runner where multiple writes must commit together. External effects that cannot participate in SQLite transactions are sequenced explicitly and made retry-safe where the current product behavior requires retries.

Versioned SQLite migration ordering remains centralized in platform bootstrap, while each context owns the SQL and compatibility tests for its tables. Existing migration identifiers and applied databases remain forward compatible; this refactor does not rename tables merely to match module names.

Alternative considered: repository methods each opening their own connection. That is simple for reads but can silently break atomic multi-write use cases.

### 7. Use typed errors internally and one command-safe mapping at the edge

Domain errors express invariant failures. Application errors add use-case categories such as not found, conflict, unsupported, and unavailable. Infrastructure errors retain diagnostic causes without exposing secrets. The Tauri interface maps them to the project's existing command-safe serialized contract and records required diagnostics through the unified logging adapter after redaction.

Domain code never writes logs. Application code may emit semantic diagnostics through an output port when the event is part of the use case; concrete persistence always uses the unified logging service. Moved operations retain their operation id and log association.

Alternative considered: preserving one root `AppError` used by every layer. It makes command serialization easy but couples domain rules to infrastructure categories and encourages logging from arbitrary locations.

### 8. Make architecture rules executable

Module visibility (`pub(super)`, `pub(crate)`, and private-by-default modules) provides the first boundary. A Rust architecture test, run by `cargo test`, will parse native source files and fail on forbidden dependency paths. It will at minimum enforce:

- no Tauri, Rusqlite, process, filesystem, HTTP, keyring, logging implementation, infrastructure, or command imports from domain modules;
- no Tauri, Rusqlite, concrete infrastructure, or command imports from application modules;
- no direct imports of another context's private modules;
- no business implementation added to `lib.rs` or command adapters.

The check will use Rust syntax parsing rather than line-based matching, and any temporary exception must be narrow, documented in the test policy, and removed in the same migration phase. Domain tests use in-memory values, application tests use fake ports, infrastructure tests use temporary SQLite/filesystem/process doubles as appropriate, and command contract tests verify serialization and error mapping.

Alternative considered: relying only on review. Review remains necessary for semantic ownership, but automated checks catch mechanical dependency regressions consistently.

### 9. Preserve external runtime boundaries

The existing frontend boundary is unchanged:

```text
React component -> frontend service interface -> Tauri adapter -> Tauri command
React component -> frontend service interface -> Web/mock adapter
```

The refactor begins behind the Rust command handler. No React component gains direct `invoke()` usage, no Tauri-only behavior enters Web adapters, and no frontend contract changes unless a separate OpenSpec change explicitly proposes one.

## Risks / Trade-offs

- [Large refactor changes many imports and visibility boundaries] -> Migrate one bounded context or vertical use case at a time, keep old and new adapters temporarily, and delete legacy code only after parity tests pass.
- [DDD layers become ceremony around simple CRUD] -> Require layers only for domain behavior or external boundaries; allow small pure helpers and cohesive modules without artificial entities or services.
- [Incorrect context boundaries create cross-context chatter] -> Record the context map and published APIs before moves, measure actual dependencies, and revise ownership through a short ADR rather than exposing internals.
- [Trait-heavy ports make Rust lifetimes and async code harder] -> Use narrow object-safe traits only at replaceable boundaries, prefer concrete types within a layer, and avoid a generic repository abstraction.
- [Mapping separate DTO, domain, and row types adds code] -> Keep explicit conversions close to the boundary and accept the mapping cost where it protects invariants or compatibility.
- [Incremental coexistence temporarily duplicates paths] -> Mark compatibility modules as migration-only, track their deletion in tasks, and forbid new behavior in legacy modules.
- [Architecture source checks produce false positives] -> Parse Rust syntax, enforce a small documented rule set, and combine the check with module privacy and focused tests.
- [Moving logging or task plumbing loses diagnostics] -> Add parity tests for redaction, operation ids, state transitions, and terminal logs before switching each long-running use case.
- [Existing SQLite data is damaged by structural cleanup] -> Do not rewrite schema for folder alignment; run empty/current/representative-old database migration tests before each persistence cutover.

## Migration Plan

1. Capture the current command/DTO/migration inventory and add characterization tests for serialized contracts, representative databases, command construction, task states, and unified logs.
2. Add the DDD standards to `openspec/project.md`, create the module skeleton, define the context map and published APIs, and enable the architecture test in `cargo test`.
3. Extract platform adapters and application-facing contracts for database access, command execution, clock/id generation, logging, tasks, filesystem/network access, and credentials without changing callers.
4. Migrate one existing tooling vertical slice end to end to validate the pattern, then migrate the remaining tooling subdomains while preserving each Tauri handler contract.
5. Migrate desktop/settings and workspace use cases, including Git/file/shell safety boundaries.
6. Migrate sessions, messages, configuration, export, maintenance, and usage persistence behind session-owned repositories and application APIs.
7. Migrate agent registry, workflow, provider invocation, and generation lifecycle; then switch IM routing to the published sessions and agent-runtime application APIs.
8. Move final Tauri handlers into context-grouped one-command files, centralize construction/registration in bootstrap, and reduce `lib.rs` to module export plus `run()` delegation.
9. Remove `RegistryStore`, root `AppError`, legacy service entry points, and temporary compatibility modules only after no callers remain and all verification passes.
10. Run the full project verification suite and strict OpenSpec validation, then document the resulting context map and any accepted follow-up ADRs before archive.

Rollback is context-scoped. Each migration phase keeps the original command contract and database schema, so a failing phase can rewire the handler to its prior implementation or revert that phase's commit without data rollback. Schema changes needed for unrelated behavior remain additive, versioned migrations and are not coupled to module moves.

## Open Questions

- Should usage reporting remain a sessions-owned read model after extraction, or become a separate reporting context? Default: keep it in `sessions` until another consumer or independent lifecycle demonstrates a real boundary.
- Which tooling subdomain should be the first reference slice? Default: MCP management, because it already has `models`, `service`, `connection`, and `commands` modules and exercises SQLite, external processes/network access, tasks, logging, and Tauri DTO mapping.
