## ADDED Requirements

### Requirement: Expanded audited builtin CLI identities

The backend SHALL add `qwen-code`, `kimi-cli`, `qoder-cli`, `codebuddy-code`, `copilot-cli`, `cursor-agent-cli`, and `iflow-cli` as stable built-in identities. Each SHALL have reviewed executable identity rules and at least one explicit distribution or detect-only source definition. Existing ids and their relative order MUST remain stable.

#### Scenario: Read the expanded catalog

- **WHEN** the application requests the built-in CLI catalog
- **THEN** the backend SHALL return all seven additions once with unique ids and metadata
- **AND** display names and UI language SHALL NOT determine identity

#### Scenario: A local-only provider has no managed installer

- **WHEN** iFlow is registered for legacy detection
- **THEN** the backend SHALL represent an explicit detect-only source without inventing package metadata or install actions

### Requirement: Executable identity validation for new providers

New providers SHALL reuse bounded source-aware discovery and validate candidate identity, launcher aliases, distribution, version, and platform before launch. A custom path MAY select an installation of a reviewed built-in provider but SHALL NOT create an arbitrary provider or trust a basename alone.

#### Scenario: Unrelated agent executable is on PATH

- **WHEN** an unrelated program named `agent` precedes Cursor in PATH
- **THEN** discovery SHALL preserve the conflict and SHALL NOT classify that program as a compatible Cursor installation

#### Scenario: Two kimi installations are discovered

- **WHEN** legacy Python and current Kimi distributions expose the same basename
- **THEN** discovery SHALL retain distinct installation identities and active versus recommended selection
- **AND** no automatic credential or session migration SHALL occur

#### Scenario: Selected executable changes

- **WHEN** the candidate fingerprint changes after preflight
- **THEN** launch SHALL revalidate identity and compatibility instead of using stale readiness

### Requirement: Read-only discovery and explicit compatibility checks

Availability discovery SHALL use only reviewed, bounded, non-interactive probes and SHALL NOT create agent sessions, open authentication pages, mutate global configuration, install packages, or send model prompts. A connection handshake with possible agent startup side effects SHALL be a separate user-initiated or explicitly authorized runtime action.

#### Scenario: Settings page opens

- **WHEN** the user opens CLI Management
- **THEN** the application SHALL show cached or read-only discovery results without spawning seven ACP sessions

#### Scenario: No documented auth status exists

- **WHEN** an adapter has no verified read-only authentication probe
- **THEN** authentication SHALL remain unknown with an explicit login or connection-check action
- **AND** version success SHALL NOT be interpreted as authenticated

#### Scenario: User requests a compatibility check

- **WHEN** the user explicitly starts a connection check
- **THEN** the system SHALL use an observable bounded operation, send no model prompt, and release temporary resources

### Requirement: Source-safe lifecycle for expanded providers

New CLI installation actions SHALL reuse persisted, expiring, single-use action plans, audited source metadata, managed installer retrieval, platform preflight, explicit target semantics, and post-mutation verification. A source SHALL NOT claim exact version installation, repair, or safe automatic execution without evidence for that action.

#### Scenario: A reviewed npm installation is requested

- **WHEN** a user confirms an npm plan for a newly supported package and exact version
- **THEN** execution SHALL use that source and version and verify the selected installation afterward

#### Scenario: A vendor script can switch installation sources

- **WHEN** a candidate vendor installer may internally fall back to npm and no source-locked execution has been verified
- **THEN** that vendor source SHALL remain guidance-only for automatic management
- **AND** the system SHALL NOT claim top-level URL filtering constrains all installer subprocess downloads

#### Scenario: Unsupported architecture is selected

- **WHEN** a provider distribution has no verified support for the host architecture
- **THEN** the backend SHALL withhold installation and launch actions requiring that support and explain the reason

### Requirement: Explicit authentication and account environment

The system SHALL provide reviewed login guidance or explicit login operations, preserve the CLI-owned credential store, and isolate account/environment selection from UI language. Only credential references and redacted status summaries SHALL be persisted or exposed.

#### Scenario: A user selects CodeBuddy China environment

- **WHEN** the user chooses the China account environment
- **THEN** the adapter SHALL use the verified environment setting for that profile regardless of interface language

#### Scenario: Authentication output includes secrets

- **WHEN** a login or diagnostic operation returns token-like values or sensitive query parameters
- **THEN** the system SHALL redact them before persistent logs and ordinary UI diagnostic rendering

#### Scenario: A login URL is supplied by an agent

- **WHEN** an agent requests opening an external login URL
- **THEN** the host SHALL validate the destination and require user interaction rather than executing arbitrary returned command text

### Requirement: Legacy iFlow local compatibility

The iFlow entry SHALL be marked legacy, show the official service shutdown date of 2026-04-17, and require explicit opt-in for a verified locally installed CLI. It SHALL NOT offer default automatic installation, automatic migration, managed ACP execution, or unattended execution in this change. User-managed custom API configuration MAY be used by the local CLI without credential import by VaneHub.

#### Scenario: No local iFlow installation exists

- **WHEN** a user views the legacy iFlow entry without a local installation
- **THEN** the UI SHALL show the shutdown context and local compatibility guidance without advertising a working official service

#### Scenario: User opts into a local installation

- **WHEN** the user selects a verified local iFlow installation and acknowledges legacy limitations
- **THEN** the runtime SHALL permit only a verified compatible terminal path and applicable policy
- **AND** it SHALL NOT change iFlow configuration or convert its history into Qoder sessions
