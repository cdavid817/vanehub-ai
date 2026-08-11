## Purpose

Defines typed, scoped, revision-safe Skill configuration while isolating secrets and supplying deterministic effective values to eligible Skill runtime contexts.

## ADDED Requirements

### Requirement: Bounded Skill configuration schema
The system SHALL accept an optional `config_schema` in Skill frontmatter using the supported JSON Schema subset. Each property SHALL have a stable key and type and MAY declare a default, constraints, enum choices, user-facing metadata, ordering, advanced visibility, and secret classification. The system MUST reject unsupported schema keywords, excessive size or nesting, unsafe references, duplicate normalized keys, and defaults that do not validate.

#### Scenario: Valid schema is discovered
- **WHEN** the effective Skill revision contains a supported configuration schema
- **THEN** the system exposes a normalized, bounded form descriptor and schema hash for that exact revision

#### Scenario: Invalid schema is discovered
- **WHEN** a schema contains an unsupported construct or invalid default
- **THEN** the system marks configuration unavailable for that revision and does not infer a permissive schema

#### Scenario: Skill has no schema
- **WHEN** an effective Skill revision declares no `config_schema`
- **THEN** the system treats the Skill as not configurable without changing its existing load behavior

### Requirement: Scoped configuration and deterministic precedence
The system SHALL support non-secret configuration at User and Project scope. Effective values SHALL resolve property by property in the order Project over User over schema default, and each effective property SHALL retain its source provenance. A Project value MUST be associated with a canonical workspace identity.

#### Scenario: Project overrides one property
- **WHEN** a Project configuration defines one property and inherits the remaining properties
- **THEN** the effective snapshot uses the Project value for that property and resolves every other property from User or schema default

#### Scenario: Workspace is unavailable
- **WHEN** Project-scoped configuration is requested without a canonical workspace
- **THEN** the system rejects the operation without falling back to an ambiguous directory or User write

#### Scenario: Scoped value is reset
- **WHEN** a user resets a property at Project scope
- **THEN** the stored Project override is removed and the effective value immediately inherits from User or schema default

### Requirement: Authoritative validation and atomic saves
The native service SHALL validate keys, types, formats, constraints, total size, scope eligibility, schema revision, and optimistic-concurrency witness before saving. A failed save MUST preserve the prior non-secret values and credential state without partially applying the request.

#### Scenario: Valid configuration is saved
- **WHEN** a request matches the current schema and expected stored revision
- **THEN** the system atomically saves the scoped values and returns the new redacted state and effective preview

#### Scenario: Stale editor submits changes
- **WHEN** a save request carries a stored revision that no longer matches
- **THEN** the system rejects it as stale and returns enough redacted state to refresh without overwriting newer values

#### Scenario: Unknown property is submitted
- **WHEN** a request contains a key not present in the exact effective schema
- **THEN** the system rejects the entire save and retains the previous configuration

### Requirement: Secret isolation and replacement semantics
Properties classified as secret SHALL be stored only through the operating-system credential store. SQLite, frontend responses, Web storage, logs, prompts, transcripts, evidence dossiers, usage records, and evolution candidates MUST NOT contain the secret value or a reversible credential reference. Read models SHALL report only configured, missing, or error state. Secret updates SHALL use replace, preserve, or clear intent rather than round-tripping an existing value.

#### Scenario: Secret-backed configuration is read
- **WHEN** the frontend requests configuration containing a stored secret
- **THEN** the response reports that the property is configured without returning its value or credential-store key

#### Scenario: Secret is preserved during non-secret edit
- **WHEN** a save changes non-secret values and marks the secret property as preserve
- **THEN** the existing credential remains unchanged

#### Scenario: Credential write fails
- **WHEN** a requested secret replacement cannot be committed atomically with non-secret state
- **THEN** the operation reports failure and restores or preserves the prior complete configuration state

### Requirement: Revision binding and schema drift
Stored configuration SHALL retain the Skill id, scope, schema hash, base Skill revision, stored revision, and validation status. When the effective schema changes, the system SHALL revalidate stored values and classify them as compatible, migration-required, or invalid; it MUST NOT silently coerce or delete incompatible values or reuse a secret under a changed incompatible declaration.

#### Scenario: Compatible schema adds an optional property
- **WHEN** a new effective schema preserves stored properties and adds an optional property with a valid default
- **THEN** existing values remain usable and the new property resolves from its default

#### Scenario: Property type changes
- **WHEN** a stored value no longer validates because its property's type changed
- **THEN** the Skill configuration enters migration-required state and that invalid value is excluded from runtime snapshots

#### Scenario: Secret classification changes
- **WHEN** a schema changes a property's secret classification
- **THEN** the system requires explicit migration and does not copy its existing value between SQLite and the credential store automatically

### Requirement: Immutable runtime configuration snapshots
Every eligible Skill load, Utility delegation, and Skill tool invocation SHALL resolve a bounded immutable configuration snapshot against the exact effective Skill/schema revision and execution context. Non-secret values MAY be exposed to the Skill instruction context; secret values MUST remain opaque and MAY only be consumed by an authorized native operation designed for that property. Missing required or migration-required configuration SHALL fail the affected Skill activation before work begins.

#### Scenario: Role Skill loads with valid configuration
- **WHEN** a Role Skill is activated with a valid effective configuration
- **THEN** its context receives a bounded non-secret configuration block and secret-presence metadata tied to the immutable snapshot

#### Scenario: Utility Skill is delegated
- **WHEN** a Utility Skill begins delegated execution
- **THEN** the child context receives the same revision-bound non-secret snapshot and cannot observe later configuration edits

#### Scenario: Required configuration is missing
- **WHEN** a required property has no effective valid value
- **THEN** activation fails with a redacted actionable error before the Skill or bundled tool performs work

### Requirement: External CLI projection is not implicit
The system SHALL NOT write Skill configuration values or secrets into third-party CLI Skill files, environment variables, command arguments, or processes unless a separately specified and supported bridge explicitly defines that projection.

#### Scenario: Configured Skill is mounted to an external CLI
- **WHEN** VaneHub mounts or binds a configured Skill to an external CLI without a configuration bridge
- **THEN** the mounted Skill content remains free of managed values and the UI identifies runtime configuration consumption as unsupported for that binding

### Requirement: Configuration lifecycle and audit
Save, reset, secret replacement/clear, schema drift, migration, archive, delete, and restore operations SHALL emit redacted structured events through unified log management. Archiving a Skill SHALL retain its configuration but disable runtime consumption; deleting a user-created Skill SHALL require an explicit decision to delete or retain scoped non-secret and credential data.

#### Scenario: Configured Skill is archived
- **WHEN** a configured Skill is archived
- **THEN** its values remain available for restoration but no new runtime snapshot is issued

#### Scenario: User deletes a configured Skill
- **WHEN** deletion is requested for a Skill with stored values or credentials
- **THEN** the system presents the data-retention choice and audits the redacted outcome

