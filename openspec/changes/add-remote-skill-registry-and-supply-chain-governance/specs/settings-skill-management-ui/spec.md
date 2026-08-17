## ADDED Requirements

### Requirement: Registry catalog surface
The Skills settings page SHALL provide a Registry surface for source selection, bounded search/filtering, verified/stale health, Skill cards, publisher/version/provenance, compatibility, content categories, installed/update/revoked state, and sanitized details. All operations SHALL use the frontend service boundary.

#### Scenario: Browse available Skills
- **WHEN** an enabled registry has verified catalog metadata
- **THEN** the page displays bounded results and distinguishes available, installed, shadowed, update-available, incompatible, and revoked entries

#### Scenario: React requests a refresh
- **WHEN** the user refreshes a registry source
- **THEN** the component calls the service interface and does not invoke the native runtime directly

### Requirement: Registry source governance UI
The page SHALL let users add, inspect, enable/disable, refresh, repair authentication, rotate trust through a verified chain, and remove eligible custom sources. Root fingerprints, source origin, credential presence, metadata versions/expiry, last verification, and errors SHALL be visible without revealing credentials.

#### Scenario: Confirm a new source root
- **WHEN** a user adds a custom source
- **THEN** a confirmation surface shows independently supplied root fingerprints and warns that endpoint content cannot establish its own trust

### Requirement: Install and version-change preview
Before install, update, downgrade, or rollback, the UI SHALL show exact source/publisher/package/Skill/version, hashes, download/expanded size, compatibility, effective shadowing impact, configuration/tool/resource changes, normalized filesystem/process/network/secret permissions, authority additions/removals, trust consequences, risk/revocation state, retained rollback state, and requested action. Confirmation SHALL be bound to the immutable preview witness.

#### Scenario: Metadata changes after preview
- **WHEN** source metadata, selected target, or installed state changes before confirmation executes
- **THEN** the operation is rejected as stale and the UI requires a refreshed preview

#### Scenario: Update requests broader permissions
- **WHEN** the selected update expands any requested authority
- **THEN** the UI identifies each expansion separately and requires a fresh explicit confirmation without reusing the prior version's approval

### Requirement: Operation progress and recovery
Registry refresh, download, validation, installation, update, rollback, uninstall, cache cleanup, and recovery SHALL expose cancellable bounded progress, redacted logs, final state, and actionable failure without blocking settings navigation.

#### Scenario: Package validation fails
- **WHEN** a download completes but package verification or extraction fails
- **THEN** the page identifies quarantine and the bounded failure reason while showing the prior installed revision as unchanged

### Requirement: Revocation experience
Critical revocations SHALL appear prominently in catalog, installed Skill details, and relevant activation surfaces with source, version, severity, reason code, freshness, and verified recovery options. The UI MUST NOT offer a revoked rollback target as eligible or hide the issue because a higher-priority Skill shadows it.

#### Scenario: Installed Skill is revoked
- **WHEN** verified metadata critically revokes an installed Registry snapshot
- **THEN** the page marks activation disabled and offers only verified update, eligible rollback, uninstall, or higher-priority override guidance

### Requirement: Honest Web registry behavior
The Tauri adapter SHALL perform native trust, cache, download, extraction, and installation operations. The Web adapter MAY expose deterministic or remote-backed catalog inspection but MUST report local Registry installation, credential storage, cache state, and filesystem changes as unsupported without a secure backend.

#### Scenario: Web user selects install
- **WHEN** no secure remote registry backend is configured
- **THEN** the UI explains that local installation is unavailable and does not fabricate progress or installed state

### Requirement: Accessible responsive registry management
Catalog cards, source controls, previews, provenance, progress, revocation warnings, and recovery actions SHALL be keyboard accessible, screen-reader labeled, usable without color alone, and remain operable without horizontal page overflow on supported narrow viewports.

#### Scenario: Install preview on narrow viewport
- **WHEN** the preview is opened at the supported narrow breakpoint
- **THEN** identity, security evidence, compatibility, changes, warning, and confirmation controls remain readable and operable
