# VaneHub AI Project Standards

## Frontend i18n

- All new or changed user-visible React UI text MUST use i18n resources instead of hard-coded literals.
- Every user-visible translation key MUST be present in both `src/i18n/locales/zh-CN.json` and `src/i18n/locales/en.json`.
- Page titles, descriptions, button labels, placeholders, status labels, notices, confirmations, modal labels, empty states, tooltips, and frontend-owned user-facing errors MUST support Simplified Chinese and English.
- Locale resources MUST stay semantically aligned: matching keys in zh-CN and en describe the same concept and action.
- User-visible date and time formatting MUST use the active application language or a locale derived from it.

Allowed literal exceptions:

- Product, provider, Agent, model, protocol, executable, npm package, command, file path, URL, log level, and stable id values MAY remain literal.
- User-provided content, backend-returned diagnostic text, and mock fixture names MAY remain literal when they represent data rather than UI labels.

Required checks:

- `src/i18n/i18n-resource-parity.test.ts` MUST pass for locale key parity.
- Frontend page changes SHOULD keep the hard-coded visible text guardrail passing, updating only the explicit allowlist for stable identifiers or fixture data.

## Frontend Visual Design

- New or changed React UI MUST prefer semantic CSS tokens and shared utility classes from `src/styles.css` over page-local hard-coded color palettes.
- The `futuristic` and `minimal` styles MUST expose equivalent semantic roles for background, foreground, panel, muted panel, border, input, primary, success, warning, danger, focus ring, and shadows.
- Shared controls SHOULD be updated through primitives in `src/components/ui/` and shared page parts before adding page-specific Tailwind class systems.
- Cards and panels SHOULD use 8px radius or less unless an existing shared primitive requires otherwise.
- Desktop management surfaces SHOULD use compact operational density: stable 8px-based spacing, readable 12-14px metadata/body text in dense panels, and no hero-scale text inside cards, sidebars, or toolbars.
- Buttons, badges, tabs, navigation rows, status labels, and compact action groups SHOULD align icons and text consistently; icon-only controls MUST provide an accessible label or translated tooltip.
- Hover, active, disabled, loading, and focus states MUST not resize controls or shift adjacent content.
- Page sections MUST avoid nested card-in-card decoration. Use cards for repeated items, dialogs, and framed tools; use full-width or unframed layouts for larger sections.
- Visual QA for substantial UI changes MUST inspect representative pages in both `futuristic` and `minimal` styles at desktop and narrow widths for overlap, clipping, unreadable contrast, and blank panels.

## Long-Running Operation Responsiveness

- Potentially time-consuming work MUST be handled asynchronously and MUST NOT block React rendering, the browser event loop, Tauri command boundaries, or the Tauri main thread.
- Refresh, download, network resource access, package operations, external command execution, MCP connection testing, SDK installation/removal, Git inspection/worktree creation, large filesystem scans, and database-heavy maintenance MUST use service-backed loading/running/terminal state instead of synchronous UI-blocking flows.
- React components MUST trigger potentially slow work through frontend service interfaces and runtime adapters; they MUST NOT call Tauri `invoke()` directly or hide runtime-specific behavior inside page components.
- Tauri adapters MUST call declared native commands that return a stable operation/task id before variable-duration native work completes.
- Web/mock adapters MUST preserve the same asynchronous service contract by simulating queued/running/succeeded/failed operation state.
- Native implementations MUST run variable-duration work through backend-managed operations/tasks and expose status, timestamps, terminal result or error, and logs through the service boundary.
- Existing data SHOULD remain visible during refresh or background operation progress; pages SHOULD show stale/refreshing state rather than replacing loaded content with a blank blocking state.
- Native diagnostics, operation output, timeouts, partial completion, and failures from long-running work MUST be associated with the operation/task and persisted through the unified logging service with redaction before disk writes.

## Rust Native DDD Architecture

The Tauri native runtime MUST remain a single-crate, domain-oriented modular monolith. Domain-bearing behavior is owned by a bounded context; technical adapters stay at the outer edge. DDD patterns are required where they protect business invariants or replaceable I/O boundaries, but MUST NOT be added as ceremony around stateless helpers.

### Bounded contexts

| Context | Ownership |
| --- | --- |
| `agent_runtime` | Agent registry, interaction modes, provider invocation, workflow state, and generation lifecycle |
| `sessions` | Sessions, messages, categories, chat configuration, export, maintenance, and usage records/read models |
| `workspaces` | Local/remote projects, worktrees, bounded file/Git inspection, and session shell lifecycle |
| `tooling` | CLI lifecycle and the MCP, SDK, extension, plugin integration, Skill, and Prompt Hook subdomains |
| `communications` | IM connector configuration, credentials, protocol adapters, routing, and delivery lifecycle |
| `desktop` | App settings, startup, data/log directory actions, floating assistant, and window/tray lifecycle |
| `operations` | Observable task lifecycle and unified diagnostic/operation logging contracts |

- Every new or materially changed native business rule, use case, persistence model, external integration, and Tauri command MUST have one owning context.
- `tooling` subdomains MUST keep separate domain models and application APIs. A subdomain MAY be promoted to a peer context through an approved architecture decision when it has independent language, lifecycle, or transaction ownership.
- Usage reporting remains a `sessions` read model while usage records are owned by assistant messages.
- Cross-context calls MUST use the owning context's published `api` facade, immutable contract, or explicit event. A context MUST NOT import another context's repository, infrastructure adapter, private aggregate, or command DTO.
- A shared kernel MUST stay minimal and contain only stable identifiers or value types with documented consumers and invariant ownership. General utilities and I/O helpers belong to `platform`, not the shared domain model.

### Target module layout

```text
src-tauri/src/
├─ lib.rs                         # module exposure and bootstrap delegation only
├─ bootstrap/                     # composition root, runtime setup, state, background jobs
│  └─ runtime.rs                  # Tauri builder and explicit dependency assembly
├─ contexts/
│  └─ <context>/
│     ├─ domain/                  # entities, values, invariants, domain errors/events
│     ├─ application/
│     │  └─ ports/                # use cases and consuming-side I/O contracts
│     ├─ infrastructure/          # SQLite/process/network/filesystem/credential adapters
│     └─ api.rs                   # deliberately published cross-context contract
├─ commands/
│  ├─ registry.rs                 # complete invoke handler grouped by bounded context
│  └─ <context>/                  # one Tauri command per file, grouped by context
└─ platform/                      # reusable outer technology adapters
   ├─ database/
   ├─ process/
   ├─ filesystem/
   ├─ logging.rs
   ├─ network/
   ├─ credentials/
   ├─ clock.rs
   └─ ids.rs
```

- Empty layer directories MUST NOT be created until the context needs them.
- Migration-only compatibility modules require an active, explicitly documented OpenSpec task. The completed DDD migration has no standing compatibility-module allowance.
- Modules are private by default. Use the narrowest practical visibility (`pub(super)` or `pub(crate)`); public context access goes through `api` or an explicit interface contract.
- `src-tauri/src/lib.rs` MUST NOT contain domain models, SQL, Tauri command implementations, external-process construction, or application orchestration.

### Dependency direction

Allowed dependencies point inward:

```text
commands / inbound adapters -> application -> domain
infrastructure -------------> application ports + domain
bootstrap ------------------> outer implementations for construction only
```

- `domain` MUST NOT depend on Tauri, Rusqlite, filesystem/process/network APIs, credential stores, task registries, logging implementations, infrastructure, commands, bootstrap, or another context's private modules.
- `application` MUST depend only on its domain, its input/output models and ports, and deliberately published cross-context contracts. It MUST NOT depend on Tauri state/commands, Rusqlite connections, or concrete I/O adapters.
- `infrastructure` implements application-owned ports and MUST NOT define business invariants.
- Tauri command handlers validate/map transport DTOs, obtain an assembled use case, invoke it, map command-safe output/errors, and perform interface-owned event emission only. They MUST NOT execute SQL, construct external processes, or decide domain policy.
- `bootstrap` is the only layer that selects concrete implementations. Tauri-managed state MAY store assembled application services but MUST NOT be used as a service locator by domain or application code.

### Models, ports, and transactions

- Domain constructors, value objects, aggregates, and domain services MUST enforce business invariants. Deserializing a command DTO or reading a database row MUST NOT bypass those invariants.
- Tauri request/response DTOs and SQLite row structs MUST remain separate from domain types whenever transport or storage semantics differ. Conversions MUST be explicit and fallible where invalid data is possible.
- Serde derives MAY appear on a domain type only when serialization is part of the domain contract; frontend aliases and SQLite column names MUST NOT leak into domain types.
- Application ports MUST be narrow and behavior-oriented (for example, `SessionRepository` or `AgentProcessGateway`). They MUST NOT expose generic CRUD, raw SQL rows, `rusqlite::Connection`, `tauri::AppHandle`, or untyped maps across a layer boundary.
- Each modifying use case MUST define its consistency boundary. Multiple owned writes that succeed or fail together MUST use one explicit atomic repository operation or transaction port.
- External effects that cannot join a SQLite transaction MUST have explicit sequencing and retry/idempotency behavior where retries are supported.
- Versioned migration ordering stays centralized in `platform::database`; each context owns its migration SQL and compatibility tests. Folder or context renaming MUST NOT trigger destructive database renames.

### Errors, logging, and operations

- Domain errors describe invariant failures; application errors add use-case categories; infrastructure errors retain diagnostic causes. One Tauri interface mapper MUST produce the command-safe serialized error contract.
- Domain code MUST NOT write logs. Application code MAY emit semantic diagnostics only through an output port.
- Native diagnostics and operation output MUST use the unified logging service with redaction. Feature-local log files remain prohibited.
- `operations` owns semantic diagnostic/operation contracts; `platform::logging` owns redacted persistence, rotation, archival, and active-directory state. Application code MUST NOT import the platform log store directly.
- Moving a long-running use case MUST preserve its stable operation id, lifecycle state, timestamps, terminal result/error, page-visible output, unified-log association, and redaction behavior.

### Verification and exceptions

- `cargo test --manifest-path src-tauri/Cargo.toml` MUST include an architecture test that parses Rust source and reports forbidden dependencies with file locations.
- Domain tests MUST run without Tauri, SQLite, live filesystems, networks, external processes, or OS credential stores.
- Application tests MUST use deterministic port doubles. Infrastructure tests MUST cover SQLite and external-adapter mapping/safety as applicable. Migrated Tauri handlers MUST have serialized DTO and command-safe error compatibility tests before the legacy path is removed.
- Rust visibility and architecture tests provide mechanical enforcement; review remains responsible for semantic context ownership and transaction boundaries.
- A temporary architecture exception MUST be narrow, justified in `src-tauri/ARCHITECTURE.md`, tied to an unchecked migration task, and removed before that context's migration is complete. Blanket or permanent allowlists are prohibited.
- The implemented boundary refinements and their rationale are recorded as ADRs in `src-tauri/ARCHITECTURE.md`; changes to those decisions require updating that document in the same proposal.
- Native refactoring MUST preserve Tauri command names, request/response serialization, supported SQLite data, frontend service interfaces, and Web/mock runtime behavior unless another approved OpenSpec change explicitly modifies them.
