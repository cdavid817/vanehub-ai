## Purpose

Defines secure remote Skill discovery and immutable Registry-layer installation with signed metadata, bounded packages, controlled updates, revocation, rollback, and offline safety.

## ADDED Requirements

### Requirement: Trusted registry sources
The system SHALL manage registry sources using stable ids, HTTPS endpoints, enabled state, trusted root metadata, optional credential-presence state, refresh health, and last verified metadata witness. Adding or replacing a custom source trust root SHALL require explicit user confirmation and MUST NOT be inferred from the endpoint response.

#### Scenario: Add a custom registry source
- **WHEN** a user submits a valid HTTPS endpoint and independently obtained root trust metadata
- **THEN** the system previews the source identity and root fingerprints before saving the enabled source

#### Scenario: Endpoint presents a different root
- **WHEN** a registry response attempts to replace the locally trusted root outside a valid signed root-rotation chain
- **THEN** the system rejects the metadata and preserves the prior trusted source state

#### Scenario: Registry credential is configured
- **WHEN** a source requires authentication
- **THEN** the credential is stored through the operating-system credential store and read models report only configured, missing, or error state

### Requirement: Signed versioned registry metadata
Every catalog refresh and package selection SHALL be authorized by threshold-signed, versioned, expiring metadata rooted in the configured source trust. Verification SHALL protect root rotation, publisher namespace delegation, target hashes and sizes, metadata rollback, mix-and-match, and freeze attacks.

#### Scenario: Metadata chain is valid
- **WHEN** root, freshness, snapshot, publisher delegation, and target metadata satisfy signatures, thresholds, versions, hashes, and expiry
- **THEN** the catalog may expose the authorized target metadata as verified

#### Scenario: Older metadata is replayed
- **WHEN** a response contains a metadata version older than the highest verified version for that source
- **THEN** the system rejects it and retains the last known verified state

#### Scenario: Metadata is expired
- **WHEN** required freshness or target authorization metadata is expired
- **THEN** the system does not use it to authorize a new install or update

### Requirement: Bounded catalog discovery
The system SHALL provide paginated, bounded catalog browsing, search, filters, details, publisher/source identity, compatibility, version history, package size, content categories, configuration/tool presence, risk indicators, install state, update state, and revocation state from verified metadata. Catalog content MUST be treated as untrusted display text and MUST NOT execute or load package instructions.

#### Scenario: Search the catalog
- **WHEN** a user searches an enabled healthy source
- **THEN** the system returns a bounded page of verified metadata with stable source, package, Skill, publisher, and version ids

#### Scenario: Catalog text contains markup
- **WHEN** publisher-provided text contains HTML, script-like content, control characters, or links
- **THEN** the UI renders sanitized plain text and permits only validated explicit links through the safe opener

### Requirement: Network and download safety
Registry traffic SHALL use the VaneHub-managed network client and active proxy/bypass settings with HTTPS, bounded redirects restricted by policy, connection/read/total timeouts, response size limits, cancellation, and no credential forwarding across unauthorized origins. Downloads SHALL stream into an application-owned quarantine cache and MUST NOT write directly into an active Skill directory.

#### Scenario: Download redirects to another origin
- **WHEN** a package request redirects to an origin not authorized by verified metadata and source policy
- **THEN** the system rejects the redirect without forwarding authorization credentials

#### Scenario: User cancels a download
- **WHEN** an install operation is cancelled during transfer
- **THEN** the transfer stops, the partial object remains in quarantine or is safely discarded, and no installed state changes

### Requirement: Bounded package verification and extraction
Before installation, the system SHALL verify the selected package against authorized identity, version, length, cryptographic hashes, and package manifest, then extract it with limits for compressed size, expanded size, file count, individual file size, path length, nesting, and compression ratio. It MUST reject absolute paths, traversal, case/Unicode collisions, reserved names, links, device entries, alternate streams, duplicate targets, unexpected executable kinds, and content outside the supported Skill package structure.

#### Scenario: Valid package reaches staging
- **WHEN** a downloaded archive matches verified metadata and every archive/package constraint
- **THEN** it is extracted to a new isolated staging directory and receives a complete content manifest and validation report

#### Scenario: Archive contains traversal
- **WHEN** an entry would escape staging after canonical path normalization
- **THEN** extraction fails before writing that entry and the package remains quarantined

#### Scenario: Package contains bundled tools
- **WHEN** a valid package declares bounded Skill tools
- **THEN** installation records their presence and hashes but does not grant executable-tool trust or operational permission

### Requirement: Explicit installation preview and atomic install
The system SHALL present exact source, publisher, Skill id, selected version, hashes, size, compatibility, content categories, schema/tool changes, risk, and effective-layer impact before install, update, downgrade, or rollback. A confirmed operation SHALL validate again and atomically publish an immutable Registry-layer snapshot plus matching database state; failure MUST preserve the prior installed/effective revision.

#### Scenario: Install a verified package
- **WHEN** the user confirms a compatible verified version and final verification succeeds
- **THEN** the system publishes exactly that immutable snapshot into the Registry layer and records its provenance lock

#### Scenario: Database commit fails after staging
- **WHEN** filesystem publication or database persistence cannot complete as one logical operation
- **THEN** the system compensates to the prior installed state and reports whether manual recovery is required

#### Scenario: Higher-priority Skill shadows the install
- **WHEN** an installed Registry Skill id is already supplied by Project or User layer
- **THEN** the preview and result identify it as installed but shadowed without replacing the higher-priority package

### Requirement: Controlled update, downgrade, rollback, and uninstall
The system SHALL check for compatible updates in the background without downloading package bodies or installing them automatically. Update, downgrade, rollback, and uninstall SHALL require explicit action and preserve immutable prior snapshots according to a bounded rollback policy. Version selection MUST NOT cross a publisher namespace or stable Skill identity silently.

#### Scenario: Update is available
- **WHEN** verified metadata advertises a newer compatible version
- **THEN** the installed Skill is marked update-available while its current snapshot remains active

#### Scenario: Update validation fails
- **WHEN** a confirmed update package fails verification or package validation
- **THEN** the existing installed revision remains active and no trust state is inherited by the rejected revision

#### Scenario: User rolls back
- **WHEN** a retained prior snapshot remains verified under current non-revoked metadata
- **THEN** the system can atomically select it after preview and confirmation

### Requirement: Revocation and emergency containment
Verified registry metadata SHALL communicate package-version revocation with severity, reason code, replacement guidance, and timestamp. A critical security revocation SHALL make the affected Registry snapshot ineligible for new effective Skill loads and bundled tool execution, cancel not-yet-started work, retain evidence and user-owned data, and present recovery actions. It MUST NOT silently switch to an unverified version.

#### Scenario: Installed version is critically revoked
- **WHEN** a refresh verifies a critical revocation for the installed version
- **THEN** the system disables new activation of that Registry snapshot and prominently identifies an allowed update, rollback, uninstall, or higher-priority override path

#### Scenario: Revocation metadata is unavailable offline
- **WHEN** the application is offline after a previously verified non-revoked package was installed
- **THEN** the installed immutable snapshot remains usable under its last verified state while the UI reports stale revocation freshness

### Requirement: Cache and offline behavior
The system SHALL maintain a bounded content-addressed cache separated from active snapshots and quarantine. Installed immutable snapshots SHALL remain usable offline. Cached catalog data MAY be shown as stale, but expired or incomplete metadata MUST NOT authorize a new install, update, downgrade, or rollback. Cache eviction MUST NOT remove active snapshots or the last rollback candidate selected by retention policy.

#### Scenario: Browse while offline
- **WHEN** verified catalog metadata is cached but the source is unreachable
- **THEN** the system shows cached entries with last-verified and stale indicators without claiming freshness

#### Scenario: Install from expired cache
- **WHEN** a cached package exists but its authorizing metadata is expired
- **THEN** the system refuses installation until trust metadata can be refreshed

### Requirement: Independent trust domains
Registry provenance verification SHALL establish only that a package is authorized by its trusted source and publisher namespace. It MUST NOT grant Overlay trust, configuration values, permission-policy grants, approval decisions, executable Skill tool trust, automatic evolution application, or trust to a higher-priority copied/forked package.

#### Scenario: Verified package includes executable Skill tools
- **WHEN** installation succeeds for a package containing modules
- **THEN** those modules remain disabled until their exact revision receives the separate tool trust required by the Skill tool runtime

#### Scenario: User forks a Registry Skill
- **WHEN** a user copies an immutable Registry package into User scope for editing
- **THEN** the new User package receives local provenance and does not inherit the Registry snapshot's publisher authorization or executable trust

### Requirement: Registry operation audit
Source changes, metadata refresh and verification, catalog use, previews, downloads, package validation, install/update/downgrade/rollback/uninstall, revocation, quarantine, cache eviction, recovery, and failures SHALL emit redacted structured events through unified log management and expose bounded operation progress in the Skills page.

#### Scenario: Authenticated refresh fails
- **WHEN** a registry request fails with credentials configured
- **THEN** diagnostics contain the source id, operation, redacted origin, status class, and error code without credentials, tokens, response bodies, or sensitive query values

