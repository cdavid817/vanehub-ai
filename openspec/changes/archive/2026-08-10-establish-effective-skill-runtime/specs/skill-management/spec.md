## MODIFIED Requirements

### Requirement: Dual Skill scopes
The system SHALL preserve `global` and `workspace` as management and binding scopes while resolving runtime Skill definitions from the `user` and `project` layers respectively, together with lower-priority `registry` and `system` layers. Scope-specific enabled state, Agent bindings, drift state, and deletion intent SHALL remain isolated.

#### Scenario: Global Skills use home scope
- **WHEN** a user lists Skills for the `global` management scope
- **THEN** the system SHALL return the effective non-project inventory and its user, registry, and system layer information

#### Scenario: Workspace Skills use project boundary
- **WHEN** a user lists Skills for the `workspace` management scope with a workspace directory
- **THEN** the system SHALL include project-layer definitions only from that canonical workspace boundary

#### Scenario: Same Skill id in different scopes
- **WHEN** the same canonical Skill id exists in both global and workspace management scopes
- **THEN** the project-layer definition SHALL be effective in that workspace
- **AND** the system SHALL manage scope-specific enabled state, Agent bindings, drift state, and deletion intent without altering the lower-layer content

### Requirement: Standard SKILL.md metadata
The system SHALL use `SKILL.md` as the required definition file for every filesystem Skill and SHALL parse a fixed frontmatter schema containing `id`, `name`, `description`, `category`, `version`, optional `triggers`, optional `aliases`, and optional `type` and `delivery`. Existing valid documents that omit the optional fields SHALL remain compatible.

#### Scenario: Valid Skill metadata
- **WHEN** a Skill directory contains a `SKILL.md` with valid required frontmatter
- **THEN** the system SHALL parse the metadata and expose it in Skill list, preview, create, edit, import, resolution, and drift responses

#### Scenario: Missing SKILL.md
- **WHEN** a Skill registry record points to a directory that does not contain `SKILL.md`
- **THEN** the system SHALL report drift for that Skill instead of treating it as healthy

#### Scenario: Immutable Skill id
- **WHEN** a user edits an existing mutable Skill
- **THEN** the system SHALL reject attempts to change the Skill `id`

#### Scenario: Legacy classification defaults
- **WHEN** a valid existing `SKILL.md` omits `type` or `delivery`
- **THEN** the system SHALL preserve its existing eager behavior by applying the compatibility classification defined by the effective Skill runtime

### Requirement: Built-in Skill seeds
The system SHALL provide the six existing built-in Skills as immutable System packages: `tdd-discipline`, `code-review`, `code-security-scan`, `api-doc-generation`, `unit-test-generation`, and `readme-generation`. Startup reconciliation SHALL migrate prior mutable built-in state idempotently, SHALL preserve user changes as User-layer overrides, SHALL preserve intentional deletion or disablement state, and SHALL report a per-Skill outcome so one failure cannot block the remaining Skills.

#### Scenario: Idempotent built-in initialization
- **WHEN** built-in Skill reconciliation runs more than once
- **THEN** the system SHALL NOT create duplicate registry records, overrides, tombstones, or Skill directories

#### Scenario: Deleted built-in is not auto-restored
- **WHEN** a user intentionally deleted a legacy built-in Skill and reconciliation runs later
- **THEN** the system SHALL preserve the deletion intent and SHALL NOT make that system Skill effective automatically

#### Scenario: Restore built-in Skill
- **WHEN** a user restores a previously deleted built-in Skill
- **THEN** the system SHALL clear the deletion intent and expose the immutable System package unless a higher-layer definition is effective
- **AND** SHALL NOT materialize a mutable copy of the System package

#### Scenario: Adopt an existing source that has no registry record
- **WHEN** reconciliation finds a legacy built-in source directory with no registry record and no deletion marker
- **THEN** the system SHALL inspect the existing source without modifying it
- **AND** SHALL preserve divergent valid content as a User-layer override
- **AND** SHALL treat content identical to the shipped definition as safely represented by the System package

#### Scenario: Adopted content that diverges from the shipped definition is reported, not overwritten
- **WHEN** existing valid built-in content differs from the immutable System definition
- **THEN** the system SHALL preserve that content as a User-layer override and report the migration outcome
- **AND** it SHALL NOT silently replace the user's files

#### Scenario: One unusable built-in does not block the rest
- **WHEN** reconciliation cannot migrate state for one built-in Skill
- **THEN** the system SHALL still reconcile every other built-in Skill it can
- **AND** it SHALL name which Skills succeeded and which failed, with a safe reason for each failure

#### Scenario: An already-present built-in is not an error
- **WHEN** reconciliation encounters an already-reconciled built-in Skill
- **THEN** the system SHALL treat the condition as an idempotent success rather than an error
- **AND** any diagnostic it emits SHALL be attributed to the reconciliation operation

#### Scenario: Unchanged legacy source cleanup
- **WHEN** a legacy mutable built-in source exactly matches the shipped System package and its state has been migrated successfully
- **THEN** the system SHALL stop treating that source as authoritative
- **AND** SHALL remove it only through a recoverable, idempotent migration step after retaining the state needed for rollback

## ADDED Requirements

### Requirement: Effective Skill lifecycle responses
Skill management list, preview, binding, enablement, drift, and restore responses SHALL identify canonical Skill id, effective layer, origin, type, delivery, availability, shadowed definitions, and compatibility state when applicable.

#### Scenario: Higher layer shadows built-in
- **WHEN** a User-layer Skill shadows a System package with the same canonical id
- **THEN** management responses SHALL identify the User definition as effective and the System definition as shadowed

#### Scenario: Unsupported Utility shown safely
- **WHEN** a Utility Skill is present before delegated execution is supported
- **THEN** management responses SHALL retain it in inventory with an unavailable reason rather than silently treating it as a Role Skill

### Requirement: Runtime adapter parity for effective Skills
The frontend service boundary, desktop adapter, and Web/mock adapter SHALL expose compatible effective Skill response shapes and operations. React components SHALL NOT call native commands directly.

#### Scenario: Desktop effective inventory
- **WHEN** the desktop UI requests the effective Skill inventory through the service boundary
- **THEN** the Tauri adapter SHALL return native runtime data using the shared frontend contract

#### Scenario: Web effective inventory
- **WHEN** the browser UI requests the effective Skill inventory through the service boundary
- **THEN** the Web/mock adapter SHALL return behaviorally representative data using the same shared contract

