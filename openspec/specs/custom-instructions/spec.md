# custom-instructions Specification

## Purpose
TBD - created by archiving change add-personalization-settings. Update Purpose after archive.
## Requirements
### Requirement: Scoped custom instructions configuration and persistence
The system SHALL provide two custom-instruction fields ("about you" and "response style") through the dedicated personalization service, SHALL support global, Agent, workspace, and workspace-Agent policy scopes, and SHALL apply them to every compatible VaneHub-managed Agent through deterministic inheritance rather than one host-wide toggle. Each field at a persisted scope SHALL be limited to 3,000 Unicode characters and SHALL be saved through an expected-revision policy patch. The system SHALL NOT transmit instruction content anywhere except the concrete generation or CLI prompt request in which the effective segment is intentionally injected.

#### Scenario: Save global custom instructions
- **WHEN** a user saves valid global about-you or response-style fields with the current global policy revision
- **THEN** the personalization service SHALL persist only the global policy scope
- **AND** the new values SHALL apply to subsequent compatible VaneHub-managed Agent generations without an application restart

#### Scenario: Save an Agent override
- **WHEN** a user saves an Agent-scope `append`, `replace`, or `disabled` override for a stable registered Agent id
- **THEN** the system SHALL persist the override independently from the global and other Agent scopes
- **AND** subsequent generations for that Agent SHALL resolve it according to personalization precedence

#### Scenario: Save a workspace or workspace-Agent override
- **WHEN** a user saves a valid override for a resolved workspace or workspace-Agent scope
- **THEN** the override SHALL apply only when that workspace context is active and, for a workspace-Agent scope, only for the selected Agent

#### Scenario: Load default custom instructions
- **WHEN** no migrated or persisted personalization policy exists
- **THEN** the system SHALL present empty custom-instruction fields
- **AND** SHALL use the personalization domain's validated safe defaults

#### Scenario: Reject an oversized field
- **WHEN** a user submits either field with more than 3,000 Unicode characters
- **THEN** the UI SHALL prevent save and show a localized inline count/error
- **AND** the native personalization boundary SHALL independently reject the patch without changing the persisted revision

#### Scenario: Reject a stale edit
- **WHEN** a custom-instruction patch carries a stale expected revision
- **THEN** the service SHALL return a typed conflict and safe current scope record
- **AND** the UI SHALL preserve the user's draft rather than replacing unrelated personalization state

#### Scenario: Migrate legacy host-level settings
- **WHEN** an existing installation first loads the dedicated personalization service
- **THEN** the system SHALL migrate the legacy fields and enablement value into the global policy idempotently
- **AND** the generic `AppSettings` fields SHALL no longer be the runtime source of truth after migration completes

### Requirement: Resolved custom instructions system-prompt section assembly
The system SHALL assemble effective, non-empty resolved custom-instruction segments into a distinct personalization section while retaining segment scope and order metadata for diagnostics. Within each scope segment, response style SHALL be ordered before about-you. When the resolved instruction mode is disabled or no effective field is non-empty, the system SHALL produce no user-personalization section. This requirement governs only user-personalization content; core instructions, role instructions, Skills, Prompt Hooks, memory, and safety instructions remain governed by their own capabilities.

#### Scenario: Assemble inherited and appended fields
- **WHEN** a generation resolves non-empty global and appended higher-precedence instruction fields
- **THEN** the section SHALL preserve scope resolution order
- **AND** SHALL place response style before about-you within each included scope segment

#### Scenario: Replace inherited fields
- **WHEN** the highest effective instruction merge mode is `replace`
- **THEN** the section SHALL contain only the replacement and any later appended user-personalization segments
- **AND** SHALL retain all non-personalization core/runtime sections

#### Scenario: Disabled produces no personalization section
- **WHEN** the effective instruction merge mode is `disabled`
- **THEN** the generation request SHALL contain no user-personalization instruction section

#### Scenario: Only one field is populated
- **WHEN** an included scope contains only one non-empty field
- **THEN** the assembled segment SHALL contain only that field and no empty placeholder

#### Scenario: Snapshot resolution fails without a safe policy
- **WHEN** no validated personalization policy can be resolved for a generation
- **THEN** the system SHALL omit the user-personalization section
- **AND** SHALL continue core instructions, Skills, Prompt Hooks, and the generation without enabling personalization implicitly

### Requirement: Web runtime scoped custom instructions parity
The Web/mock runtime SHALL implement the same scoped policy, inheritance, validation, expected-revision, conflict, effective-preview, and session-mode contracts as the desktop runtime without accessing SQLite, contacting a real provider, or launching a real CLI process. Fixed simulated chat responses are not required to vary according to hidden assembled prompt text.

#### Scenario: Web mock saves scoped instructions
- **WHEN** custom instructions are created or updated through the Web/mock adapter
- **THEN** the adapter SHALL preserve the selected scope, merge mode, fields, revision, and conflict semantics through the same `AgentService` contract

#### Scenario: Web mock previews effective instructions
- **WHEN** an effective preview is requested for a registered mock Agent and workspace
- **THEN** the adapter SHALL return deterministic contributing scopes and final instruction state equivalent to desktop policy resolution

#### Scenario: No simulated prompt-content divergence
- **WHEN** custom instructions are enabled, disabled, appended, or replaced during a mock message send
- **THEN** the fixed simulated response MAY remain unchanged
- **AND** this SHALL NOT be treated as a service-contract parity failure

### Requirement: Resolved custom instructions CLI prompt injection
The system SHALL prepend effective non-empty resolved custom-instruction segments to the Prompt-Hook-assembled effective prompt for every message sent through a compatible VaneHub-managed CLI runtime adapter. Coverage SHALL be derived from the stable Agent registry and runtime capability metadata rather than a fixed list. This delivery mechanism SHALL NOT modify the CLI's native instruction files or internal conversation compaction.

#### Scenario: Effective custom instructions precede Prompt Hook assembly
- **WHEN** a CLI message snapshot contains effective custom-instruction segments
- **THEN** the final text delivered to the CLI process SHALL contain those segments before the Prompt-Hook-assembled content

#### Scenario: Apply scoped instructions on every turn
- **WHEN** a CLI session sends multiple messages or changes active workspace context
- **THEN** each turn SHALL capture and prepend its own effective snapshot
- **AND** SHALL NOT rely on a one-time first-turn injection

#### Scenario: Disabled or empty produces no injection
- **WHEN** the effective instruction mode is disabled or all effective fields are empty
- **THEN** the CLI adapter SHALL deliver the existing Prompt-Hook-assembled content without a user-personalization section

#### Scenario: Resolution failure does not block a CLI response
- **WHEN** custom-instruction resolution fails and no validated policy is available
- **THEN** the system SHALL send the Prompt-Hook-assembled content without custom instructions
- **AND** SHALL surface only a safe warning without failing or materially delaying the CLI message

#### Scenario: Prompt Hook template input remains original
- **WHEN** custom instructions are prepended for a CLI Agent
- **THEN** Prompt Hook template variables representing the user message SHALL continue to use the user's original input rather than the prepended text

#### Scenario: Dynamic compatible CLI Agent
- **WHEN** a newly registered CLI Agent declares custom-instruction support and uses the shared CLI adapter
- **THEN** it SHALL receive effective custom instructions without a new Agent-specific personalization branch

