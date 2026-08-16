## ADDED Requirements

### Requirement: Mechanically enforced native dependency direction
Native architecture fitness SHALL parse production Rust sources and reject domain or application dependencies on forbidden outer technologies or layers, and reject cross-context access to private modules.

#### Scenario: Domain imports infrastructure
- **WHEN** a domain module imports its own infrastructure layer, a concrete platform adapter, Tauri, SQLite, filesystem, process, or network APIs
- **THEN** native architecture fitness SHALL fail with the native dependency rule id, file, line, and dependency path

#### Scenario: Application imports an outer layer
- **WHEN** an application module imports concrete infrastructure, command state, Tauri, or a concrete SQLite connection
- **THEN** native architecture fitness SHALL fail with the native dependency rule id, file, line, and dependency path

#### Scenario: Context imports another context privately
- **WHEN** one bounded context imports another context's repository, infrastructure, private aggregate, or other non-API module
- **THEN** native architecture fitness SHALL fail and direct the caller to the owning context's published API, contract, or event

### Requirement: Mechanically enforced native adapter thinness
Native architecture fitness SHALL reject Tauri command handlers that execute SQL, construct external processes, or contain business-policy control flow, and SHALL reject concrete runtime I/O assembly outside bootstrap.

#### Scenario: Command performs concrete I/O
- **WHEN** a Tauri command contains SQL, opens a concrete database connection, or constructs or executes an external process
- **THEN** native architecture fitness SHALL fail with the command-thinness rule id and exact source location

#### Scenario: Concrete runtime is assembled outside bootstrap
- **WHEN** production native code outside bootstrap constructs a reviewed concrete runtime dependency that belongs to dependency assembly
- **THEN** native architecture fitness SHALL fail with the composition-root rule id and exact source location

### Requirement: Native architecture rules prove both outcomes
Native dependency, cross-context, command-thinness, and composition-root detectors SHALL each have syntax-valid positive and negative fixtures.

#### Scenario: Native fixture suite runs
- **WHEN** the native architecture test target executes
- **THEN** compliant fixtures SHALL be accepted and each prohibited dependency or I/O pattern SHALL be rejected with its rule id and location

