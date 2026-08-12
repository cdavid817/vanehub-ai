## Purpose

Provides a reversible and auditable customization layer that can safely change effective Skill instructions and supporting resources without modifying authoritative Skill packages.

## ADDED Requirements

### Requirement: Versioned Overlay document
The system SHALL represent Skill customization as a versioned Overlay document addressed by canonical Skill id and Overlay scope. The document SHALL contain base identity and hash witnesses, revision, trust metadata, exact patches, learned-guidance blocks, supporting files, conflicts, and timestamps without copying the complete base package.

#### Scenario: Create first Overlay
- **WHEN** a user creates an Overlay for an available effective Skill with no existing Overlay in that scope
- **THEN** the system SHALL create revision one with the current canonical Skill id, base identity, base content hash, trusted local origin, and requested mutation
- **AND** SHALL leave the authoritative Skill package unchanged

#### Scenario: Unsupported document version
- **WHEN** the system reads an Overlay document with an unsupported future schema version
- **THEN** it SHALL mark the Overlay unavailable and SHALL NOT replay or rewrite it

#### Scenario: Overlay does not duplicate base
- **WHEN** an Overlay is persisted
- **THEN** it SHALL contain only mutation data and base witnesses rather than a complete replacement of `SKILL.md`

### Requirement: Scoped Overlay resolution
The system SHALL support System, User, and Project Overlay scopes. It SHALL replay trusted active Overlays in deterministic order from System to User to Project for the active canonical workspace. A System-scope Overlay SHALL be global customization state and SHALL NOT make an immutable System-layer Skill package mutable.

#### Scenario: Project Overlay has final precedence
- **WHEN** a Skill has active trusted System, User, and matching Project Overlays
- **THEN** the system SHALL replay System mutations first, User mutations second, and Project mutations last

#### Scenario: Project isolation
- **WHEN** Project Overlays for the same Skill exist in two workspaces
- **THEN** each workspace SHALL replay only its own Project Overlay after applicable System and User Overlays

#### Scenario: No active workspace
- **WHEN** effective Skill content is requested without an active workspace
- **THEN** the system SHALL exclude all Project Overlays

#### Scenario: Shadowed base definition
- **WHEN** Skill layer resolution changes the effective base definition
- **THEN** the system SHALL evaluate every applicable Overlay against the new effective base witnesses before replay

### Requirement: Exact Overlay patches
An Overlay patch SHALL identify an exact `old_string`, replacement `new_string`, `replace_all` flag, creation witness, and state of `active`, `disabled`, or `reverted`. Active patches SHALL replay deterministically in creation order and SHALL fail closed when their match contract is not satisfied.

#### Scenario: Apply unique exact patch
- **WHEN** an active patch has `replace_all` disabled and its `old_string` occurs exactly once in the current replay content
- **THEN** the system SHALL replace that occurrence with `new_string`

#### Scenario: Unique patch has multiple matches
- **WHEN** an active patch has `replace_all` disabled and its `old_string` occurs more than once
- **THEN** the system SHALL NOT apply the patch and SHALL create an unresolved conflict

#### Scenario: Replace all matches
- **WHEN** an active patch has `replace_all` enabled and its `old_string` occurs one or more times
- **THEN** the system SHALL replace every exact occurrence deterministically

#### Scenario: Patch target missing
- **WHEN** an active patch's `old_string` does not occur in the current replay content
- **THEN** the system SHALL NOT apply the patch and SHALL create an unresolved conflict

#### Scenario: Disabled or reverted patch
- **WHEN** a patch is disabled or reverted
- **THEN** it SHALL remain in history but SHALL NOT change effective content

### Requirement: Learned-guidance blocks
The system SHALL store user-learned guidance as independently addressable blocks with active, disabled, or reverted state. Active trusted blocks SHALL be appended to an explicitly delimited `User-learned guidance overlay` section after successful patch replay.

#### Scenario: Append active guidance
- **WHEN** an applicable trusted Overlay contains active learned-guidance blocks
- **THEN** effective instructions SHALL contain one delimited learned-guidance section with blocks ordered by Overlay scope and creation order

#### Scenario: Preserve base wording
- **WHEN** a learned-guidance block is added
- **THEN** it SHALL NOT require or perform an exact replacement in the base instruction body

#### Scenario: Disable guidance
- **WHEN** a user disables a learned-guidance block using the current Overlay revision
- **THEN** subsequent effective content SHALL omit that block while retaining its audit history

### Requirement: Non-executable Overlay files
The system SHALL allow bounded Overlay files only under `references`, `templates`, and `assets`. It SHALL reject executable, script, binary-executable, absolute, hidden, traversing, or package-escaping paths, and SHALL expose accepted files through the effective Skill resource view according to their media type.

#### Scenario: Add reference document
- **WHEN** a user adds a bounded text file at `references/team-guidance.md`
- **THEN** the system SHALL persist it in the Overlay and SHALL let the effective resource index resolve it according to Overlay precedence

#### Scenario: Override supporting file
- **WHEN** a higher-precedence Overlay file has the same logical path as a lower Overlay or base supporting file
- **THEN** the higher-precedence file SHALL be effective and the lower file SHALL remain inspectable as shadowed

#### Scenario: Reject executable extension
- **WHEN** an Overlay file uses an executable or script extension including `.py`, `.sh`, `.bat`, `.cmd`, `.ps1`, `.exe`, `.com`, `.dll`, `.msi`, or `.wasm`
- **THEN** the system SHALL reject the mutation without persisting file content

#### Scenario: Reject unsafe path
- **WHEN** an Overlay file path is absolute, contains a parent or hidden component, targets an unsupported top-level directory, or resolves outside its Overlay storage boundary
- **THEN** the system SHALL reject the mutation without reading or writing the escaped target

#### Scenario: Non-executable asset accepted
- **WHEN** an Overlay file under `assets` has an allowed non-executable media type and passes size, signature, extension, and path validation
- **THEN** the system SHALL accept it as a binary asset while keeping it unavailable to text-only instruction reads

#### Scenario: Executable signature refused
- **WHEN** an Overlay file has an executable signature or executable media type despite using an allowed-looking filename
- **THEN** the system SHALL reject it without persisting the payload

### Requirement: Overlay trust
Locally created Overlays SHALL be trusted after passing content and path validation. Imported Overlays SHALL be untrusted and SHALL NOT affect effective instructions or resources until a user explicitly promotes the exact imported revision after reviewing its diff and scan results.

#### Scenario: Local Overlay trusted
- **WHEN** a user creates an Overlay through a local VaneHub mutation operation and validation succeeds
- **THEN** the resulting revision SHALL be trusted and eligible for replay

#### Scenario: Imported Overlay quarantined
- **WHEN** a valid Overlay package is imported
- **THEN** it SHALL be stored as untrusted, excluded from replay, and presented with its source, hashes, scan result, and diff for review

#### Scenario: Promote unchanged import
- **WHEN** a user promotes an untrusted imported Overlay using its current revision and content hash
- **THEN** the system SHALL mark that exact revision trusted and eligible for replay

#### Scenario: Imported content changed before promotion
- **WHEN** an imported Overlay's revision or hash differs from the reviewed witness at promotion time
- **THEN** the system SHALL reject promotion and require a fresh review

### Requirement: Injection and secret-content scanning
Every Overlay instruction mutation, learned block, supporting file, import, and edited reconciliation result SHALL be scanned before persistence or trust promotion. Content matching hard-deny secret or prompt-injection patterns SHALL be rejected with a safe reason and SHALL NOT be written to active Overlay state.

#### Scenario: Private key material detected
- **WHEN** submitted content contains recognizable private-key material
- **THEN** the system SHALL reject it without storing or logging the secret content

#### Scenario: Prompt override pattern detected
- **WHEN** submitted content contains a hard-deny prompt override pattern or executable script markup
- **THEN** the system SHALL reject it and return a safe rule identifier

#### Scenario: Safe literal guidance
- **WHEN** submitted content passes all deterministic content and path checks
- **THEN** scanning SHALL permit the mutation to continue to revision and witness validation

### Requirement: Compare-and-swap Overlay mutations
Every Overlay mutation SHALL require the caller's expected Overlay revision and expected effective base hash. The system SHALL perform validation, history append, Overlay persistence, and usage-counter updates as one recoverable operation or return a conflict without partially applying the mutation.

#### Scenario: Current witnesses accepted
- **WHEN** expected revision and base hash match live state and all validation succeeds
- **THEN** the system SHALL persist exactly one next revision and its history event

#### Scenario: Stale Overlay revision
- **WHEN** the submitted expected revision does not match the live Overlay revision
- **THEN** the system SHALL reject the mutation without changing Overlay state and SHALL return the current revision

#### Scenario: Base changed during edit
- **WHEN** the submitted base hash no longer matches the effective base package
- **THEN** the system SHALL reject the mutation, mark reconciliation needed, and preserve the submitted content only in the caller's unsaved UI state

#### Scenario: Persistence interrupted
- **WHEN** an Overlay mutation is interrupted before its transaction commits
- **THEN** recovery SHALL produce either the complete previous revision or the complete next revision with its corresponding history event, never a partial mixture

### Requirement: Base drift and reconciliation
The system SHALL compare each Overlay's base witnesses with the current effective base and expose `base_hash_changed` and `needs_reconcile` states. An Overlay requiring reconciliation SHALL NOT be replayed into agent-visible content until every active mutation has a deterministic resolution.

#### Scenario: Base unchanged
- **WHEN** all applicable Overlay base witnesses match the effective base and replay succeeds
- **THEN** the system SHALL expose Overlay status as healthy

#### Scenario: Base changed but all mutations replay
- **WHEN** the base hash changes and every active mutation can replay deterministically against the new base
- **THEN** the system SHALL present a reconciliation preview and require an explicit user confirmation before updating witnesses

#### Scenario: Base change creates conflict
- **WHEN** one or more active mutations cannot replay against the new base
- **THEN** the system SHALL record unresolved conflicts, mark the Overlay as needing reconciliation, and use unmodified base content for that Overlay scope

#### Scenario: Resolve conflict by editing
- **WHEN** a user edits a conflicted mutation into a valid form and confirms reconciliation against current witnesses
- **THEN** the system SHALL create a new Overlay revision, mark the prior conflict resolved, and replay the new revision

#### Scenario: Ignore conflicted mutation
- **WHEN** a user explicitly ignores a conflict
- **THEN** the conflict SHALL remain auditable as ignored and its mutation SHALL become disabled in the new revision

### Requirement: Pinned Skill refusal
A pinned Skill SHALL retain replay of Overlays already active when it was pinned but SHALL reject Overlay creation, import, promotion, patch, guidance, file, trust, disable, revert, and reconciliation mutations until the Skill is explicitly unpinned.

#### Scenario: Existing Overlay on pinned Skill
- **WHEN** a Skill with a healthy active Overlay becomes pinned
- **THEN** the current Overlay revision SHALL continue to determine effective content

#### Scenario: Mutation refused while pinned
- **WHEN** any Overlay mutation is requested for a pinned Skill
- **THEN** the system SHALL reject it with a pinned refusal and SHALL NOT change revision, history, or usage counters

### Requirement: Bounded Overlay storage and imports
The system SHALL enforce a 1 MiB limit for each Overlay supporting file, a 4 MiB limit for each active history segment, and an 8 MiB limit for an imported Overlay package. It SHALL also enforce bounded mutation counts, instruction sizes, archive entry counts, and decompressed import size.

Imported Overlay packages SHALL use the version-one ZIP profile. The archive root SHALL contain exactly one `overlay.json` manifest and MAY contain content-addressed payloads at `payloads/sha256/<lowercase-sha256>`. Every file entry other than the manifest SHALL be referenced by exactly one manifest resource, and every manifest resource SHALL resolve to exactly one matching payload entry. Import SHALL reject duplicate names, undeclared entries, missing payloads, hash or size mismatches, directories containing data outside this layout, symbolic links, hard links, encrypted entries, absolute or traversing paths, unsupported compression methods, and trailing package data.

The importer SHALL inspect and extract only into a unique quarantine staging directory. It SHALL parse the manifest, validate the complete entry set and reference closure, scan every instruction mutation and payload, and verify media, size, and content hashes before creating durable state. Imported trust metadata SHALL never be inherited: an accepted document SHALL be rewritten as origin `Imported`, state `Untrusted`, with no reviewed revision or reviewed content hash.

#### Scenario: Supporting file exceeds limit
- **WHEN** an Overlay supporting file exceeds 1 MiB
- **THEN** the system SHALL reject it before committing an Overlay revision

#### Scenario: Import archive expands beyond limit
- **WHEN** an imported package's compressed or decompressed content exceeds its configured limit
- **THEN** the system SHALL stop processing and reject the import without persisting partial entries

#### Scenario: ZIP package has a closed payload set
- **WHEN** a version-one ZIP package contains `overlay.json` and every `payloads/sha256/<lowercase-sha256>` entry is referenced exactly once with matching size and hash
- **THEN** the importer SHALL complete validation and scanning in quarantine before making an untrusted revision durable

#### Scenario: ZIP package contains an undeclared entry
- **WHEN** a ZIP package contains an entry outside `overlay.json` and the exact referenced payload set
- **THEN** the system SHALL reject the complete import and remove its quarantine staging directory

#### Scenario: Imported manifest claims trust
- **WHEN** an otherwise valid imported manifest declares local origin, trusted state, or prior review witnesses
- **THEN** the stored imported revision SHALL replace those claims with imported untrusted metadata and SHALL remain excluded from replay

#### Scenario: History segment reaches limit
- **WHEN** appending an event would exceed the active 4 MiB history-segment limit
- **THEN** the system SHALL close the existing append-only segment, start a new ordered segment, and preserve verification linkage between them

### Requirement: Append-only Overlay history
Every successful Overlay mutation and every failed replay that changes conflict state SHALL append a redacted event containing event id, canonical Skill id, scope, prior and next revision, actor, action, timestamp, hashes, and safe outcome. Existing events SHALL NOT be edited or deleted by normal Overlay operations.

#### Scenario: Successful patch event
- **WHEN** a patch mutation commits
- **THEN** history SHALL contain one linked event identifying the revision transition and content hashes without copying secret-bearing instruction bodies

#### Scenario: Revert remains auditable
- **WHEN** a user reverts an active mutation
- **THEN** the system SHALL create a new revision and append a revert event rather than deleting the original mutation or event

#### Scenario: History verification
- **WHEN** history is requested
- **THEN** the system SHALL verify segment and event linkage and SHALL report corruption without silently repairing or omitting unverifiable events

### Requirement: Overlay usage tracking
Successful patch mutations SHALL increment `patch_count`, and every successful Overlay state mutation SHALL increment `overlay_mutation_count` and its timestamp. Rejected or rolled-back operations SHALL NOT increment either counter.

#### Scenario: Patch committed
- **WHEN** an exact patch is committed successfully
- **THEN** the system SHALL increment both patch and Overlay mutation counts once

#### Scenario: Guidance committed
- **WHEN** a learned-guidance mutation commits successfully
- **THEN** the system SHALL increment the Overlay mutation count once without incrementing patch count

#### Scenario: Mutation rejected
- **WHEN** scanning, witness validation, pinning, limits, or persistence rejects a mutation
- **THEN** usage counters SHALL remain unchanged
