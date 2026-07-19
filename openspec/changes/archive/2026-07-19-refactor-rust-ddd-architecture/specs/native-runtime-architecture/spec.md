## ADDED Requirements

### Requirement: Native bounded-context ownership
The Rust native runtime SHALL organize domain-bearing behavior into documented bounded contexts, and each business rule, use case, persistence model, and integration SHALL have one explicit owning context.

#### Scenario: Add native domain behavior
- **WHEN** a new native capability or business rule is implemented
- **THEN** it SHALL be assigned to a named bounded context with vocabulary and ownership consistent with that context
- **AND** it SHALL NOT be added to a root module or generic utility module solely for cross-feature convenience

#### Scenario: Use behavior owned by another context
- **WHEN** one bounded context needs behavior or data owned by another bounded context
- **THEN** it SHALL use the owning context's published application API, immutable contract, or explicit event
- **AND** it SHALL NOT import the other context's repository, infrastructure adapter, private aggregate, or Tauri command DTO

### Requirement: Inward native dependency direction
Native context dependencies MUST point inward from interface and infrastructure adapters to application use cases and domain code, while domain and application layers SHALL remain independent of concrete runtime frameworks.

#### Scenario: Compile domain code
- **WHEN** a bounded context's domain layer is compiled or tested
- **THEN** it SHALL NOT depend on Tauri, SQLite, filesystem, network, external-process, OS credential, task-registry, or logging implementations
- **AND** it SHALL NOT depend on command, infrastructure, bootstrap, or another context's private modules

#### Scenario: Compile application code
- **WHEN** a bounded context's application layer is compiled or tested
- **THEN** it SHALL depend only on its domain model, explicit application ports, and deliberately published cross-context contracts
- **AND** it SHALL NOT depend on Tauri state, Tauri commands, Rusqlite connections, or concrete filesystem, network, process, credential, logging, or task adapters

#### Scenario: Implement an external integration
- **WHEN** SQLite, filesystem, network, process, credential, unified-log, task, or desktop-runtime behavior is required by a use case
- **THEN** an outer infrastructure or interface adapter SHALL implement a narrow port owned by the consuming application layer

### Requirement: Explicit domain and boundary models
The native runtime SHALL model business invariants in domain types and SHALL keep domain models distinct from Tauri transport DTOs and SQLite row representations whenever transport or persistence concerns differ from domain semantics.

#### Scenario: Accept a Tauri command payload
- **WHEN** a Tauri command receives serialized input
- **THEN** the interface adapter SHALL validate and map that payload into application or domain input before executing business behavior
- **AND** deserialization alone SHALL NOT bypass domain invariants

#### Scenario: Load persisted domain state
- **WHEN** an infrastructure repository reads SQLite rows
- **THEN** it SHALL map those rows into valid domain types through explicit conversion
- **AND** SQLite column details SHALL NOT become domain-layer dependencies

#### Scenario: Reject an invalid state transition
- **WHEN** a requested mutation violates a domain invariant
- **THEN** the domain or application layer SHALL return a typed error without performing the invalid persistence or external side effect

### Requirement: Use-case and port boundaries
Native business workflows SHALL be exposed as application use cases with context-specific ports, and external entry points SHALL delegate to those use cases without implementing business or persistence logic.

#### Scenario: Handle a Tauri command
- **WHEN** a declared Tauri command is invoked
- **THEN** its handler SHALL map transport input, invoke one or more explicit application use cases, map command-safe output or errors, and perform only interface-owned event emission
- **AND** it SHALL NOT execute SQL, construct external processes, or decide domain policy directly

#### Scenario: Define a persistence port
- **WHEN** an application use case needs persisted state
- **THEN** it SHALL depend on a behavior-oriented, context-owned repository or transaction port
- **AND** that port SHALL NOT expose raw SQL rows, a Rusqlite connection, or generic CRUD as the cross-layer contract

#### Scenario: Execute an atomic mutation
- **WHEN** a use case requires multiple owned persistence changes to succeed or fail together
- **THEN** its application and infrastructure boundaries SHALL preserve one explicit atomic transaction boundary

### Requirement: Native composition root
The native runtime SHALL construct concrete repositories, gateways, use cases, and interface state in a dedicated composition root, while `lib.rs` SHALL remain limited to module exposure and delegation to native bootstrap.

#### Scenario: Start the Tauri desktop runtime
- **WHEN** native setup completes storage and migration initialization
- **THEN** the composition root SHALL construct concrete adapters and inject them into assembled application services registered with Tauri state
- **AND** domain and application code SHALL NOT resolve dependencies from Tauri state or a global service locator

#### Scenario: Register Tauri commands
- **WHEN** the native application builds its invoke handler
- **THEN** command registration SHALL be centralized and auditable by bounded-context command group
- **AND** each command implementation SHALL remain in the native interface layer

### Requirement: DDD refactor compatibility
The native DDD migration SHALL preserve existing external contracts, persisted data, unified logging, and observable operation behavior unless a separate approved specification explicitly changes them.

#### Scenario: Migrate a native command
- **WHEN** an existing Tauri command is moved behind a new context use case
- **THEN** its command name, request and response serialization, command-safe error behavior, and frontend service contract SHALL remain compatible

#### Scenario: Open an existing database after migration
- **WHEN** a user starts the refactored runtime with an existing supported SQLite database
- **THEN** all previously valid data SHALL remain readable through versioned migrations
- **AND** module or context renaming SHALL NOT cause destructive schema changes

#### Scenario: Migrate a logged long-running operation
- **WHEN** an operation or task implementation moves to the new architecture
- **THEN** its stable operation id, lifecycle state, terminal result or error, available page output, unified-log association, and redaction behavior SHALL remain available

#### Scenario: Use the browser runtime during native refactor
- **WHEN** the application runs through the Web/mock adapter
- **THEN** it SHALL remain usable without importing or invoking the refactored Rust runtime

### Requirement: Executable native architecture verification
The project SHALL enforce native dependency boundaries through Rust visibility and an automated architecture check included in the normal Rust test workflow.

#### Scenario: Introduce a forbidden dependency
- **WHEN** domain or application source imports a forbidden outer-layer framework, adapter, or private cross-context module
- **THEN** the automated architecture check SHALL fail with the source location and violated dependency rule

#### Scenario: Verify a bounded context
- **WHEN** native tests run for a migrated context
- **THEN** domain tests SHALL run without live infrastructure, application tests SHALL use deterministic port doubles, and infrastructure tests SHALL cover its SQLite or external-adapter mappings where applicable

#### Scenario: Verify interface compatibility
- **WHEN** a Tauri handler is migrated
- **THEN** contract tests SHALL verify its serialized DTO shape and command-safe error mapping before the legacy path is removed

### Requirement: Project-level Rust DDD standards
The project standards SHALL document the native bounded-context map, target module layout, layer responsibilities, allowed dependency direction, model-mapping rules, port and transaction conventions, error and logging boundaries, testing expectations, and exception process.

#### Scenario: Implement new Rust native work
- **WHEN** an implementation task adds or materially changes native domain behavior
- **THEN** it SHALL follow the DDD rules in `openspec/project.md` and place the behavior in its owning bounded context

#### Scenario: Request a temporary architecture exception
- **WHEN** a migration cannot immediately comply with one documented dependency rule
- **THEN** the exception SHALL be narrow, justified, recorded with an owning migration task, and removed before that context's migration is considered complete

#### Scenario: Review a migrated context
- **WHEN** a bounded-context migration is submitted for completion
- **THEN** reviewers SHALL be able to identify its domain model, application use cases and ports, outer adapters, public context API, transaction ownership, and verification coverage from the documented structure
