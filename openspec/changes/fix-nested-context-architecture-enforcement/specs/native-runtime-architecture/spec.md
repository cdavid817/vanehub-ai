## MODIFIED Requirements

### Requirement: Complete native dependency enforcement
The native architecture test MUST inspect domain, application, infrastructure, and command source files and SHALL reject private cross-context infrastructure dependencies outside the composition root. Scope resolution MUST cover both flat `contexts/<context>/<layer>` sources and nested `contexts/<parent>/<subdomain>/<layer>` sources on every supported path separator. A source file that the test cannot place into a scope MUST NOT be silently skipped.

#### Scenario: Infrastructure imports another context repository
- **WHEN** a bounded context infrastructure module imports another context's concrete repository
- **THEN** the architecture test SHALL fail with the importing file, line, and dependency

#### Scenario: Command executes infrastructure behavior
- **WHEN** a command handler imports or invokes a context's private infrastructure implementation
- **THEN** the architecture test SHALL fail unless the command uses a deliberately published API contract

#### Scenario: Nested subdomain source is inspected
- **WHEN** a source file lives under a context subdomain such as `contexts/tooling/<subdomain>/domain`
- **THEN** the architecture test SHALL resolve its context, subdomain, and layer and apply every dependency rule to it

#### Scenario: Nested subdomain reaches its own outer layer
- **WHEN** a subdomain's domain module imports that same subdomain's application or infrastructure layer
- **THEN** the architecture test SHALL fail with the native dependency rule id and source location

#### Scenario: Path separators differ by platform
- **WHEN** the same nested source path is presented with forward slashes and with backslashes
- **THEN** scope resolution SHALL produce the same context, subdomain, and layer

### Requirement: Mechanically enforced native dependency direction
Native architecture fitness SHALL parse production Rust sources and reject domain or application dependencies on forbidden outer technologies or layers, and reject cross-context access to private modules. Forbidden network technology SHALL be identified semantically: socket, listener, and address-resolution APIs are forbidden, while address value types that perform no I/O are permitted.

#### Scenario: Domain imports infrastructure
- **WHEN** a domain module imports its own infrastructure layer, a concrete platform adapter, Tauri, SQLite, filesystem, process, or network APIs
- **THEN** native architecture fitness SHALL fail with the native dependency rule id, file, line, and dependency path

#### Scenario: Application imports an outer layer
- **WHEN** an application module imports concrete infrastructure, command state, Tauri, or a concrete SQLite connection
- **THEN** native architecture fitness SHALL fail with the native dependency rule id, file, line, and dependency path

#### Scenario: Context imports another context privately
- **WHEN** one bounded context imports another context's repository, infrastructure, private aggregate, or other non-API module
- **THEN** native architecture fitness SHALL fail and direct the caller to the owning context's published API, contract, or event

#### Scenario: Domain parses a network address
- **WHEN** a domain or application module imports an address value type such as `IpAddr`, `Ipv4Addr`, or `Ipv6Addr` to validate or classify a declared origin
- **THEN** native architecture fitness SHALL accept it, because parsing an address opens no socket and reaches no network

#### Scenario: Domain opens a socket
- **WHEN** a domain or application module imports a socket, listener, or address-resolution API such as `TcpStream`, `TcpListener`, `UdpSocket`, or `ToSocketAddrs`
- **THEN** native architecture fitness SHALL fail with the native dependency rule id and source location

## ADDED Requirements

### Requirement: Every consumed context publishes a cross-context API
A bounded context consumed by another context SHALL expose its cross-context contract through a published `api` module. A consumer SHALL NOT import the owning context's application, domain, or infrastructure modules directly, and enforcement SHALL NOT be waived by a file-level or path-level exemption list.

#### Scenario: A context gains a second consumer
- **WHEN** a context that has no published `api` module is imported by another context
- **THEN** it SHALL publish one and the consumer SHALL depend on that module instead of its internals

#### Scenario: Enforcement is bypassed by an exemption
- **WHEN** a change would add a file or path exemption so an existing cross-context violation stops being reported
- **THEN** the violation SHALL be repaired instead, and the architecture rules SHALL keep reporting it until it is
