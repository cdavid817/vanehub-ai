# native-developer-documentation Specification

## Purpose
TBD - created by archiving change establish-multilingual-documentation. Update Purpose after archive.
## Requirements
### Requirement: Curated mdBook developer guide
The repository SHALL provide an English mdBook developer guide covering system architecture, frontend service boundaries, native bounded contexts, persistence and migration ownership, unified logging, testing, packaging, and contribution workflows.

#### Scenario: Developer navigates the guide
- **WHEN** a developer builds or opens the developer guide
- **THEN** its summary SHALL expose the documented architecture and maintenance topics through stable chapter paths
- **AND** existing authoritative repository documents SHALL be linked or included instead of copied into conflicting narratives

#### Scenario: Explain runtime differences
- **WHEN** a documented behavior differs between Tauri desktop and Web/mock runtime
- **THEN** the guide SHALL identify both behaviors and SHALL preserve the React service and adapter boundary

### Requirement: Complementary Rustdoc API reference
The documentation build SHALL generate a Rustdoc reference from the native crate with dependencies excluded and private items included, while preserving existing Rust visibility and architecture.

#### Scenario: Build native API reference
- **WHEN** the documentation build runs with the supported stable Rust toolchain
- **THEN** it SHALL run the equivalent of `cargo doc --no-deps --document-private-items` for `src-tauri/Cargo.toml`
- **AND** Rustdoc warnings configured as documentation failures SHALL fail the build

#### Scenario: Document selected native boundaries
- **WHEN** a maintainer opens the generated API reference
- **THEN** the crate entry and selected context APIs, domain contracts, application ports, and command boundary types SHALL contain purpose, invariant, error, or ownership documentation appropriate to their role
- **AND** implementation visibility SHALL NOT be widened solely to make an item appear in Rustdoc

#### Scenario: Navigate between guide and reference
- **WHEN** the assembled documentation output is built
- **THEN** the mdBook developer guide SHALL link to the Rustdoc root under a stable sibling path
- **AND** the Rustdoc output SHALL remain reference material rather than duplicated Markdown chapters

### Requirement: Reproducible documentation build
The repository SHALL expose a single documented build entry point that produces the developer guide, localized user guides, and native API reference in a deterministic output tree using pinned documentation tooling.

#### Scenario: Build documentation locally
- **WHEN** a maintainer runs the documented build command with declared prerequisites installed
- **THEN** it SHALL build every book and the native API reference
- **AND** generated output SHALL be placed in an ignored directory without modifying authored documentation

#### Scenario: Build documentation in CI
- **WHEN** CI evaluates a documentation change
- **THEN** it SHALL install pinned documentation-only tooling, run the same repository build entry point, and upload or retain the assembled site as a CI artifact
- **AND** no frontend or native application runtime dependency SHALL be added for documentation generation

### Requirement: Documentation validation
The documentation pipeline SHALL validate Markdown links, README parity, mdBook navigation, supported Rust code samples, Rustdoc generation, and documentation output assembly.

#### Scenario: Detect a broken internal link
- **WHEN** an authored guide or assembled output references a missing repository-relative chapter or asset
- **THEN** the documentation check SHALL fail and identify the source file and invalid target

#### Scenario: Validate code samples
- **WHEN** `mdbook test` encounters a Rust sample marked as testable
- **THEN** the sample SHALL compile and execute according to mdBook semantics
- **AND** illustrative or environment-dependent samples SHALL be explicitly marked rather than failing unpredictably

#### Scenario: Keep selected API boundaries documented
- **WHEN** a selected native documentation boundary gains or changes an exposed contract item
- **THEN** the documentation checks SHALL require the boundary inventory and its Rust documentation to remain complete
