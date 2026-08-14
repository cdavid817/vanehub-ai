## Purpose

Defines immutable, content-addressed Artifacts that let OnePiece preserve, inspect, publish, download, and relate bounded outputs without exposing arbitrary host paths.

## ADDED Requirements

### Requirement: Immutable content-addressed Artifact records
The system SHALL seal every admitted Artifact as immutable bytes plus a versioned metadata record containing stable id, content hash, byte size, media type, safe display name, creator, creation time, provenance, visibility, and lifecycle state. A published Artifact's bytes and identity-bearing metadata SHALL NOT be edited in place.

#### Scenario: Seal new content
- **WHEN** an eligible producer supplies content that passes admission
- **THEN** the system SHALL atomically persist the bytes and metadata and return the stable Artifact id and content hash

#### Scenario: Same content is sealed twice
- **WHEN** deduplication is enabled and admitted bytes match existing content
- **THEN** the storage layer MAY reuse the content blob but SHALL preserve distinct provenance records when the logical Artifact creations differ

#### Scenario: Stored bytes are altered
- **WHEN** a later read detects a content-hash mismatch
- **THEN** the system SHALL quarantine the Artifact from publication and consumption and return an integrity failure

### Requirement: Artifact admission and safe names
Artifact creation SHALL enforce hard limits for bytes, count, media type, filename length, and operation quotas. Display names SHALL be normalized as labels only; they SHALL NOT determine storage paths. The system SHALL reject traversal, absolute paths, alternate data streams, control-character ambiguity, links, special files, and unsupported executable content.

#### Scenario: Producer supplies a traversal filename
- **WHEN** an Artifact candidate uses `..`, an absolute path, a reserved device path, or another unsafe name
- **THEN** the system SHALL reject or replace the display label according to policy without writing outside managed storage

#### Scenario: Producer exceeds its Artifact budget
- **WHEN** an operation attempts to seal more bytes or items than its effective quota
- **THEN** the system SHALL reject excess candidates and explicitly report the limit

### Requirement: Application-owned publication
Publishing an Artifact SHALL make it available through VaneHub's authenticated application surfaces and bounded service endpoints; it SHALL NOT upload content to a public or third-party host, create a provider-authored URL, or expose its managed filesystem path. Publication SHALL bind the Artifact hash, visibility scope, and reviewable metadata.

#### Scenario: Publish to the originating session
- **WHEN** an admitted Artifact is published with session visibility
- **THEN** the chat and Artifact surfaces SHALL expose an application-owned reference that resolves only through the authorized service boundary

#### Scenario: Caller requests a public Internet URL
- **WHEN** no separately specified external publishing provider exists
- **THEN** the system SHALL reject public upload rather than silently sending the Artifact to an external service

### Requirement: Bounded preview, read, and download
Authorized users SHALL be able to inspect metadata, preview supported content, page through admitted text, and download original bytes through the service boundary. Preview generation SHALL be bounded and isolated, unsupported content SHALL not execute, and downloads SHALL revalidate integrity and authorization.

#### Scenario: Preview supported text
- **WHEN** a caller reads a permitted text Artifact
- **THEN** the system SHALL return bounded paged content with explicit truncation and stable provenance

#### Scenario: Preview active or unsupported content
- **WHEN** an Artifact could execute active content or has no safe previewer
- **THEN** the system SHALL show metadata/download controls without executing it in the application origin

#### Scenario: Download integrity fails
- **WHEN** the managed bytes no longer match the Artifact hash
- **THEN** the system SHALL block download and mark the integrity failure

### Requirement: Artifact lineage and evidence roles
Artifacts derived from Browser, Web, code execution, OCR, or CLI delegation SHALL record their producing operation and parent Artifact ids/hashes where applicable. The system SHALL distinguish model/provider claims from host-verified evidence and SHALL not elevate an Artifact's trust merely because OnePiece or an external CLI produced it.

#### Scenario: OCR creates a text Artifact
- **WHEN** OCR output is sealed
- **THEN** its lineage SHALL identify the source Artifact and OCR operation without copying unrestricted source content into logs

#### Scenario: Delegation creates a ChangeSet
- **WHEN** an edit delegation is successfully sealed
- **THEN** its Artifact metadata SHALL identify the delegation attempt, exact base commit, diff hash, and host-computed evidence role

### Requirement: Retention and referential cleanup
The system SHALL apply bounded retention and storage quotas without deleting an Artifact that remains referenced by a retained message, operation, delegation, apply attempt, or other retained Artifact lineage. Cleanup SHALL be idempotent, crash-safe, and report orphaned or corrupt storage without exposing content.

#### Scenario: Retention expires an unreferenced Artifact
- **WHEN** an unpinned, unreferenced Artifact exceeds retention policy
- **THEN** the system MAY remove its logical record and unshared blob through recoverable cleanup

#### Scenario: Blob is shared or still referenced
- **WHEN** a candidate blob remains referenced by any retained Artifact or evidence record
- **THEN** cleanup SHALL preserve the blob

### Requirement: Artifact service boundary parity
Artifact operations SHALL be exposed through shared frontend service interfaces with Tauri and Web/mock implementations. React SHALL not receive raw managed paths, and Web/mock SHALL use bounded in-memory fixtures or unsupported results without writing host files.

#### Scenario: Desktop opens an Artifact
- **WHEN** a React surface requests an Artifact preview
- **THEN** the Tauri adapter SHALL call the native service and return only the shared DTO

#### Scenario: Web mock publishes an Artifact
- **WHEN** a mock workflow publishes deterministic content
- **THEN** Web/mock SHALL return a clearly simulated reference and SHALL not claim durable native storage

