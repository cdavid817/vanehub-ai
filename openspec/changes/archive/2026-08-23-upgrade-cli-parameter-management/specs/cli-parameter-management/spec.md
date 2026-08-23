## MODIFIED Requirements

### Requirement: Managed CLI parameter profiles

The system SHALL provide one typed launch-parameter profile for each external managed CLI stable agent id: `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, and `antigravity-cli`. Each returned profile SHALL identify the active catalog version, persisted profile revision, explicit parameter selections, cached executable status, compatibility diagnostics, and safe saved-profile previews for supported launch scopes.

#### Scenario: List managed profiles

- **WHEN** the CLI Parameter Management page loads
- **THEN** the system SHALL return profiles for the five external managed stable agent ids in their configured display order
- **AND** each profile SHALL contain catalog definitions, explicit selections, catalog defaults, profile revision, catalog version, cached executable metadata, diagnostics, and safe argument previews

#### Scenario: Exclude native Agents from CLI profiles

- **WHEN** the CLI Parameter Management page lists managed profiles
- **THEN** it SHALL NOT represent OnePiece or another native API Agent as an empty external CLI profile
- **AND** it SHALL direct users to the owning Agent Configuration surface for native Agent settings

#### Scenario: Return a missing or conflicting executable state

- **WHEN** the existing CLI lifecycle read model reports that a managed CLI is missing, unrunnable, version-unsupported, or affected by multiple-installation conflict
- **THEN** the profile SHALL include that cached state and an actionable diagnostic
- **AND** loading or editing the profile SHALL NOT start an executable probe

#### Scenario: Reject unknown agent profile

- **WHEN** a client requests, previews, saves, or resets a parameter profile for an unknown agent id
- **THEN** the service SHALL reject the request without persisting any selection

### Requirement: Typed and documented parameter catalog

Every exposed CLI parameter SHALL be defined by a canonical backend-authoritative capability-registry entry with a stable parameter id, literal provider mapping, ownership, category, maturity, control kind, localized name and description keys, explicit inherited/default semantics, launch scope, risk classification, validation constraints, deterministic render strategy, argument placement, compatibility metadata, dependency and conflict rules, and allowed or bounded dynamic values when applicable.

#### Scenario: Render composite model parameter

- **WHEN** a catalog entry has control kind `custom-text` with known values
- **THEN** the page SHALL render an inherited choice, a dropdown containing the known values plus a localized Custom option, and a controlled free-text input when Custom is selected
- **AND** selecting Custom alone SHALL NOT write an empty selection or replace the last valid value

#### Scenario: Select known catalog model value

- **WHEN** a user selects a known catalog model value
- **THEN** the draft SHALL contain an explicit typed value selection
- **AND** the service SHALL validate and render it using the same registry entry used by the native runtime

#### Scenario: Enter custom model value

- **WHEN** a user selects Custom and enters a value that satisfies the parameter-specific constraints
- **THEN** the system SHALL preserve the normalized custom value as an explicit typed selection
- **AND** it SHALL NOT require the value to appear in the known-option list

#### Scenario: Render enum parameter

- **WHEN** a catalog entry has control kind `enum`
- **THEN** the page SHALL render an inherited choice and a single-select control using only values currently allowed by the evaluated registry entry
- **AND** it SHALL show localized value descriptions and compatibility state

#### Scenario: Render boolean parameter

- **WHEN** a catalog entry has control kind `boolean`
- **THEN** the page SHALL distinguish inherited behavior from an explicit true or false value when the render strategy supports both states
- **AND** it SHALL expose an accessible control whose labels describe the emitted behavior rather than an implementation-specific stored value

#### Scenario: Render mutually exclusive positive and negative flags

- **WHEN** a registry entry represents an inherited, positive-flag, or negative-flag choice
- **THEN** the page SHALL render one tri-state control
- **AND** the renderer SHALL emit at most one of the mutually exclusive flags

#### Scenario: Render repeatable enum parameter

- **WHEN** a catalog entry has control kind `multi-enum`
- **THEN** the page SHALL render an inherited choice and an accessible multi-select control using only the evaluated allowed values
- **AND** the service SHALL preserve catalog-defined or user-defined order according to the entry's ordering rule

#### Scenario: Render ordered string list

- **WHEN** a catalog entry accepts an ordered list of bounded text values
- **THEN** the page SHALL support adding, editing, removing, and reordering items within the catalog constraints
- **AND** the renderer SHALL apply the entry's declared repeated-token or joined-value strategy deterministically

#### Scenario: Render path list

- **WHEN** a catalog entry accepts a bounded list of directories
- **THEN** the page SHALL support explicit directory selection and normalized manual values through approved service adapters
- **AND** it SHALL identify duplicates and missing paths without recursively scanning those paths

### Requirement: Explicit profile save and reset

The CLI Parameter Management page SHALL maintain isolated per-CLI draft state and SHALL persist changes only through an explicit profile save or reset action. Mutations SHALL use the baseline profile revision and catalog version to prevent stale or semantically outdated writes.

#### Scenario: Edit profile draft

- **WHEN** a user changes a parameter control
- **THEN** the page SHALL mark that CLI profile as having unsaved changes
- **AND** navigation to another CLI profile SHALL preserve each profile's isolated draft while the page remains mounted
- **AND** every CLI with a dirty draft SHALL retain a visible dirty count or badge in the navigation rail

#### Scenario: Preserve custom input by CLI and parameter

- **WHEN** two managed CLIs expose the same stable parameter id and the user enters a custom value for one CLI
- **THEN** switching to the other CLI SHALL NOT reuse, display, or save the first CLI's transient custom input

#### Scenario: Save valid profile

- **WHEN** a user saves a valid CLI profile with the current profile revision and catalog version
- **THEN** the service SHALL validate and persist the complete selection set in one transaction or equivalent atomic Web/mock update
- **AND** the page SHALL replace its baseline with the returned profile, clear the dirty state, and retain the selected launch scope

#### Scenario: Reject a stale profile revision

- **WHEN** a user saves a draft whose baseline revision is older than the persisted profile revision
- **THEN** the service SHALL reject the mutation with a structured revision-conflict error
- **AND** it SHALL NOT overwrite the newer profile
- **AND** the page SHALL offer reload and explicit draft-discard actions rather than silently applying last-write-wins behavior

#### Scenario: Reject a stale catalog version

- **WHEN** a user saves selections prepared against a catalog version that is no longer accepted
- **THEN** the service SHALL reject the mutation with a structured catalog-version conflict
- **AND** it SHALL return enough information for the page to reload and revalidate the draft

#### Scenario: Restore defaults

- **WHEN** a user confirms Restore Inherited Values for one CLI with the current profile revision
- **THEN** the service SHALL atomically remove that CLI's persisted overrides or replace them with explicit inherited selections according to the persistence contract
- **AND** the page SHALL show inherited behavior without changing another CLI profile

#### Scenario: Leave with unsaved changes

- **WHEN** the user attempts to leave the settings route while any CLI draft is dirty
- **THEN** the shared unsaved-change guard SHALL identify that unsaved CLI profiles exist
- **AND** the user SHALL be able to remain on the page or discard those in-memory drafts

### Requirement: Runtime-specific persistence parity

The desktop runtime SHALL persist versioned CLI parameter profiles and revisions in SQLite, and the Web/mock runtime SHALL preserve the same service semantics using namespaced browser-local storage without claiming to launch local CLIs. Both runtimes SHALL apply the same legacy-selection migration and structured diagnostic rules.

#### Scenario: Restore desktop selections

- **WHEN** the desktop application restarts after a valid profile was saved
- **THEN** the CLI profile SHALL be restored from SQLite with its revision and selection-schema version
- **AND** it SHALL be returned through the frontend service boundary without frontend access to SQLite

#### Scenario: Restore Web mock selections

- **WHEN** the Web runtime reloads after a valid mock profile was saved
- **THEN** the Web adapter SHALL restore the versioned profile from namespaced browser storage
- **AND** it SHALL NOT access SQLite or a local executable

#### Scenario: Load a valid legacy selection

- **WHEN** an existing profile contains a legacy value that can be mapped unambiguously to inherited or typed selection semantics
- **THEN** the service SHALL expose the migrated selection without data loss
- **AND** the first successful save or reset SHALL rewrite that profile using the current selection schema

#### Scenario: Load an invalid or unknown legacy selection

- **WHEN** an existing profile contains malformed JSON, an unknown parameter, or a value incompatible with the current registry
- **THEN** the service SHALL quarantine that row from argument rendering and return a repair diagnostic
- **AND** it SHALL NOT silently reinterpret the row or prevent other valid selections from loading

#### Scenario: Retry an interrupted migration

- **WHEN** profile migration is executed more than once after a previous process interruption
- **THEN** the migration SHALL be idempotent
- **AND** a failed profile rewrite SHALL leave the previously committed profile and revision intact

### Requirement: Provider-specific argument injection

The native runtime SHALL resolve logical selections through the tooling context's published CLI-parameter API, convert the resolved selections into distinct argv token segments, and place those segments according to the selected provider's interactive, fresh-chat, and resume command grammar. Provider builders SHALL NOT read CLI-parameter persistence or duplicate catalog rendering rules.

#### Scenario: Start interactive CLI

- **WHEN** the user launches an interactive managed CLI with compatible saved parameters applicable to the `interactive` scope
- **THEN** the native runtime SHALL inject the validated rendered tokens in the registry-declared argument slots before spawning the process
- **AND** it SHALL omit incompatible selections and associate their diagnostics with the attempted launch

#### Scenario: Start fresh chat CLI

- **WHEN** a new chat generation starts a provider CLI process with compatible saved parameters applicable to the `chat` scope
- **THEN** the native runtime SHALL inject the resolved token segments while preserving the provider's required structured-output and prompt-delivery contract

#### Scenario: Resume provider session

- **WHEN** a chat generation resumes a provider session
- **THEN** the provider builder SHALL place resolved profile segments only in slots accepted by the resume grammar
- **AND** it SHALL preserve the native session id and stdin or prompt contract

#### Scenario: Render a provider-specific configuration override

- **WHEN** a registry entry uses a declarative provider configuration renderer instead of a simple flag-value pair
- **THEN** the runtime SHALL render that selection from registry metadata without branching on the parameter id
- **AND** it SHALL pass the resulting value as one argv token wherever the provider grammar requires one token

### Requirement: Deterministic configuration precedence

For an ordinary logical parameter supported by the active provider, the native runtime SHALL resolve an explicit per-message value before a persisted VaneHub profile value, and SHALL resolve an explicit profile value before inherited provider behavior. An inherited profile selection means VaneHub emits no user-profile token. Policy-governed execution, approval, automatic-approval, permission, and sandbox values SHALL be resolved exclusively by their owning policy path and SHALL take final precedence.

#### Scenario: Message value overrides persisted default

- **WHEN** a chat message supplies a supported non-security value that is also explicitly saved in the CLI profile
- **THEN** the provider invocation SHALL use the message value for that process
- **AND** the persisted profile SHALL remain unchanged

#### Scenario: No message override

- **WHEN** a chat message does not supply a supported non-security value and the profile has an explicit compatible value
- **THEN** the provider invocation SHALL use the saved profile value

#### Scenario: Inherited selection emits no profile token

- **WHEN** a parameter selection is inherited and no message override exists
- **THEN** VaneHub SHALL emit no user-profile token for that parameter
- **AND** the provider SHALL retain responsibility for its own default and configuration layers

#### Scenario: Provider value named default remains explicit

- **WHEN** a provider accepts the literal string `default` as an actual parameter value and the registry declares it as such
- **THEN** the service SHALL preserve and render that explicit value
- **AND** it SHALL NOT confuse that provider value with VaneHub's inherited state

#### Scenario: Policy overrides a security parameter

- **WHEN** a launch resolves an effective execution policy
- **THEN** its execution, approval, automatic-approval, permission, and sandbox arguments SHALL come from the owning policy projection
- **AND** neither a message nor a saved CLI parameter profile SHALL override them

### Requirement: Safe effective argument preview

The settings page SHALL obtain draft and saved-profile previews through the service boundary for an explicit `chat` or `interactive` scope. A preview SHALL expose individual validated user-controlled argv tokens, placement segments, and diagnostics; it SHALL omit prompts, session identifiers, secrets, runtime-owned protocol values, and policy-owned tokens, and SHALL never be represented as an executable shell command.

#### Scenario: Display preview after save

- **WHEN** a profile is loaded or successfully saved
- **THEN** the page SHALL display the service-returned saved-profile preview for the selected launch scope
- **AND** it SHALL display every token as a distinct indexed or otherwise unambiguous argv item

#### Scenario: Preview an unsaved draft

- **WHEN** the user changes a valid draft selection
- **THEN** the page SHALL request a debounced draft preview for the active CLI and selected scope
- **AND** only the newest matching preview response SHALL replace the current draft preview

#### Scenario: Switch preview scope

- **WHEN** the user switches between Chat and Interactive preview
- **THEN** the service SHALL render the same draft using the selected launch scope
- **AND** parameters that do not apply to that scope SHALL be omitted with no loss of the draft selection

#### Scenario: Keep previous preview during refresh

- **WHEN** a new preview request is in progress and a previous valid preview exists
- **THEN** the page SHALL retain the previous preview with a refreshing indication
- **AND** it SHALL NOT replace the preview panel with an empty blocking state

#### Scenario: Copy safe preview

- **WHEN** a user copies the preview
- **THEN** the page SHALL offer a JSON argv representation or an explicitly display-only escaped representation
- **AND** it SHALL NOT label a cross-platform string as a command to execute

#### Scenario: Preview contains a token with whitespace

- **WHEN** one rendered argv value contains spaces or shell metacharacters
- **THEN** the preview SHALL still represent it as one token
- **AND** the service SHALL NOT split or shell-evaluate that value

### Requirement: Localized and theme-consistent page

The CLI Parameter Management page SHALL use all registered locale resources and shared semantic settings tokens in both `futuristic` and `minimal` themes. The information architecture SHALL remain compact, searchable, accessible, and usable at supported narrow widths without nested decorative card hierarchies.

#### Scenario: Switch locale

- **WHEN** the page renders in any registered locale
- **THEN** page labels, parameter descriptions, option descriptions, compatibility badges, diagnostics, validation states, filters, preview labels, and actions SHALL use the active locale
- **AND** literal CLI flags, executable paths, provider names, versions, and stable ids MAY remain untranslated

#### Scenario: Switch theme

- **WHEN** the active theme changes between `futuristic` and `minimal`
- **THEN** navigation, controls, descriptions, badges, diagnostics, preview tokens, and action states SHALL remain readable and usable without page-specific theme branches

#### Scenario: Browse at a supported narrow width

- **WHEN** the page is displayed at a supported narrow desktop width
- **THEN** the CLI navigation, parameter groups, preview, and sticky actions SHALL reflow without horizontal page overflow
- **AND** no required parameter action SHALL become pointer-only

#### Scenario: Filter parameters

- **WHEN** the user searches or selects All, Modified, Warnings, Unsupported, or Advanced
- **THEN** the page SHALL filter using localized labels, descriptions, literal flags, stable ids, and option text as applicable
- **AND** it SHALL announce the result count and preserve focus when possible

### Requirement: Agent Terminal uses interactive profile only

The Agent Terminal runtime SHALL request the selected Agent's resolved CLI Parameter profile through the tooling context's published API with the `interactive` launch scope. It SHALL use only compatible non-security token segments from that resolution and SHALL resolve execution, approval, automatic-approval, permission, and sandbox behavior through the Agent policy path.

#### Scenario: Start terminal with interactive profile

- **WHEN** an Agent Terminal process starts for a managed CLI stable agent id
- **THEN** the native runtime SHALL load and resolve that agent id's saved profile for the `interactive` scope
- **AND** it SHALL inject only compatible non-security token segments returned by the tooling API

#### Scenario: Ignore removed chat controls

- **WHEN** an Agent Terminal process is built
- **THEN** it SHALL use the Agent policy directly for governed behavior
- **AND** it SHALL NOT read a session execution mode as a CLI profile value

#### Scenario: Profile changes affect next terminal process

- **WHEN** a CLI Parameter profile is saved while a retained Agent Terminal process is live
- **THEN** the live process SHALL continue with its original ordinary arguments
- **AND** the next process SHALL use the newly saved compatible profile values

#### Scenario: Policy template overrides a governed parameter

- **WHEN** an Agent Terminal starts for any managed CLI
- **THEN** the launch SHALL use values projected from the Agent policy for every execution, approval, automatic-approval, permission, or sandbox parameter

#### Scenario: Installed version no longer supports a saved value

- **WHEN** the cached active executable version makes a saved interactive value unsupported
- **THEN** the Agent Terminal SHALL omit that value rather than pass a known-invalid token
- **AND** the launch operation SHALL record a structured compatibility diagnostic

### Requirement: Custom-text parameter control kind

The parameter catalog SHALL support a controlled `custom-text` control kind that combines inherited state, known values, and an optional free-text editor. The editor SHALL use parameter-specific normalization and constraints, and transient editor mode SHALL remain separate from a valid transport selection.

#### Scenario: Validation accepts known enum values

- **WHEN** a `custom-text` parameter receives a value matching one of the currently allowed known entries
- **THEN** validation SHALL accept the value

#### Scenario: Validation accepts arbitrary non-empty values

- **WHEN** a `custom-text` parameter receives a value not in the known entries
- **THEN** validation SHALL accept it only when the entry permits custom values and all length, format, normalization, and character constraints pass

#### Scenario: Validation rejects control characters

- **WHEN** a custom value contains a disallowed control character or disallowed bidirectional formatting character
- **THEN** validation SHALL reject the value with a structured field diagnostic

#### Scenario: Validation rejects empty values

- **WHEN** Custom is selected but the input is empty or whitespace-only
- **THEN** the page SHALL retain an invalid editor state and disable save for that profile
- **AND** it SHALL NOT submit an empty typed value or silently convert the draft to a provider value

#### Scenario: Restore inheritance from a custom value

- **WHEN** a user chooses Inherit after editing a custom value
- **THEN** the valid selection SHALL become inherited
- **AND** the renderer SHALL emit no user-profile token for that parameter

### Requirement: Argument preview renders custom model values directly

When a compatible `custom-text` model selection contains a custom value, the service preview SHALL preserve that value as one argv token under the declared provider rendering and scope. Inherited state SHALL be represented separately from provider values.

#### Scenario: Custom model value in argument preview

- **WHEN** Claude Code has an explicit model value `deepseek-chat` and the preview is generated for a compatible scope
- **THEN** the preview SHALL contain distinct tokens `--model` and `deepseek-chat`

#### Scenario: Known model value in argument preview (unchanged)

- **WHEN** Claude Code has an explicit known model value `sonnet` and the preview is generated
- **THEN** the preview SHALL contain distinct tokens `--model` and `sonnet`

#### Scenario: Inherited model value is omitted from preview

- **WHEN** any managed CLI has an inherited model selection and the preview is generated
- **THEN** the preview SHALL omit the user-profile model mapping entirely

#### Scenario: Default model value omitted from preview (unchanged)

- **WHEN** a legacy profile stored the historical sentinel model value `default` for a registry entry that does not declare `default` as a real provider value
- **THEN** the migrated selection SHALL be inherited
- **AND** the preview SHALL omit the user-profile model mapping entirely

#### Scenario: Literal provider value named default is rendered

- **WHEN** a registry explicitly permits the literal model value `default` and the user selects that provider value rather than Inherit
- **THEN** the preview SHALL render the declared model flag and the literal `default` value

### Requirement: Antigravity CLI parameter catalog

The backend-authoritative editable catalog SHALL define only officially verified Antigravity CLI parameters for model selection, reasoning effort, and agent selection. Execution mode, terminal sandbox, prompt transport, output format, conversation identity, and dangerous bypass flags SHALL remain runtime-owned or policy-owned and SHALL NOT be editable profile parameters.

#### Scenario: Load the Antigravity parameter catalog

- **WHEN** the `antigravity-cli` parameter catalog is loaded for settings
- **THEN** every exposed model, effort, or agent mapping SHALL have a current audit record and deterministic renderer
- **AND** the catalog SHALL omit a candidate whose spelling or grammar cannot be confirmed

#### Scenario: Managed invocation arguments are absent from the catalog

- **WHEN** the `antigravity-cli` editable catalog is loaded
- **THEN** it SHALL NOT contain prompt, structured-output, or conversation-identity arguments

#### Scenario: The permission bypass flag is absent from the catalog

- **WHEN** the `antigravity-cli` editable catalog is loaded
- **THEN** it SHALL NOT contain mode, sandbox, automatic-approval, permission-bypass, or similarly governed arguments

#### Scenario: Preview reflects saved selections

- **WHEN** a user selects a compatible non-inherited Antigravity effort value
- **THEN** the returned safe argument preview SHALL include only the verified effort mapping and value tokens

### Requirement: Audited user-editable CLI parameter catalog

The user-editable CLI capability registry SHALL match the current verified launch arguments and meanings for Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI while policy-governed and runtime-reserved arguments remain excluded. Desktop and Web/mock runtimes SHALL expose contract-equivalent generated definitions from the same canonical registry version.

#### Scenario: Compare frontend and native catalogs

- **WHEN** registry contract verification runs
- **THEN** the native catalog and generated TypeScript artifact SHALL expose identical agent ids, parameter ids, controls, ownership, categories, maturity, launch scopes, inherited defaults, render metadata, constraints, compatibility, dependencies, known values, and risk semantics
- **AND** a stale or manually edited generated artifact SHALL fail verification

#### Scenario: Describe a managed parameter

- **WHEN** a managed parameter is displayed in any supported locale
- **THEN** its label and description SHALL state the effect of the actual emitted provider mapping
- **AND** known values SHALL be guidance rather than an unsupported promise when the provider accepts a valid bounded custom identifier

#### Scenario: Keep policy controls single-sourced

- **WHEN** an argument controls approval, automatic approval, permissions, sandboxing, or another Agent policy concern
- **THEN** the CLI Parameters page SHALL omit that argument from editable definitions and user-controlled preview tokens
- **AND** the page SHALL link to Agent Policies for the governed behavior

#### Scenario: Verify registry invariants

- **WHEN** the canonical registry is loaded in tests or at application bootstrap
- **THEN** duplicate ids or flags, invalid defaults, unresolved localization keys, cyclic dependencies, contradictory compatibility ranges, unsafe ownership, and invalid render strategies SHALL fail registry validation

### Requirement: Officially audited comprehensive editable catalogs

The editable parameter registry for each managed CLI SHALL cover the current useful non-secret launch options verified from that CLI's official command or configuration reference, except for VaneHub-owned arguments and approval, automatic-approval, permission, or sandbox controls governed elsewhere. Each registry audit SHALL record an official source identifier, review date, reviewed provider version or documentation state, and confidence or verification status.

#### Scenario: Audit every managed CLI

- **WHEN** the registry contract is verified
- **THEN** Claude Code, Codex CLI, Gemini CLI, OpenCode, and Antigravity CLI SHALL each have an official-source audit record
- **AND** every exposed mapping SHALL match the documented spelling, value grammar, scope, and argument placement or be explicitly version-gated

#### Scenario: Official reference documents a useful safe launch option

- **WHEN** an official reference exposes a non-secret option that can be represented by supported typed controls and does not conflict with a reserved or policy-owned concern
- **THEN** the managed profile SHALL expose or explicitly defer that option with a stable audit decision
- **AND** an exposed option SHALL have a stable id, localized description, constraints, inherited behavior, risk, compatibility, and deterministic rendering

#### Scenario: Official documentation is incomplete or disagrees with an installed build

- **WHEN** a candidate mapping cannot be confirmed by an official reference or an accepted supported invocation grammar
- **THEN** the system SHALL omit it from the editable registry rather than infer or pass an unverified raw argument
- **AND** the audit record SHALL state that the candidate requires review

#### Scenario: Option is gated by version or platform

- **WHEN** an official reference establishes a minimum version, maximum version, platform, command, or model constraint
- **THEN** the registry SHALL encode that constraint
- **AND** the UI and runtime resolution SHALL use the same evaluated compatibility result

### Requirement: Expanded catalog controls remain safe and usable

Expanded CLI parameter profiles SHALL preserve atomic validation, compact presentation, safe preview behavior, stale-data continuity, and accessible operation as the number and complexity of definitions grows.

#### Scenario: Browse an expanded profile

- **WHEN** a managed CLI contains multiple parameter groups
- **THEN** the page SHALL group controls by registry category, omit empty groups, keep flag descriptions scannable, and remain usable at supported narrow widths without horizontal page overflow

#### Scenario: Identify modified and incompatible profiles

- **WHEN** one or more inactive CLI profiles have unsaved changes, warnings, errors, or unsupported selections
- **THEN** the navigation rail SHALL display persistent counts or badges for those profiles
- **AND** selecting a badge-bearing profile SHALL expose the relevant diagnostics

#### Scenario: Preview expanded selections

- **WHEN** the user changes or saves expanded parameter selections
- **THEN** the preview SHALL include only validated compatible user-controlled tokens in provider-defined segment and token order
- **AND** it SHALL continue to omit credentials, prompts, session identifiers, output-protocol arguments, and policy-governed flags

#### Scenario: Preview service is temporarily unavailable

- **WHEN** a draft preview fails after a valid preview has been displayed
- **THEN** the page SHALL retain the previous valid preview, show a scoped diagnostic, and keep unrelated profile controls available

## ADDED Requirements

### Requirement: Explicit inherited and typed parameter selections

Every parameter selection SHALL use an explicit envelope that distinguishes inherited behavior from a typed provider value. The transport and persistence contracts SHALL NOT encode inheritance by overloading strings such as `default`, boolean false, empty arrays, or absent UI-local state.

#### Scenario: Persist inherited state

- **WHEN** a user saves a profile with a parameter set to Inherit
- **THEN** the persisted selection SHALL record inherited state or omit the override according to the versioned persistence contract
- **AND** loading the profile SHALL reconstruct inherited state without consulting UI heuristics

#### Scenario: Persist an explicit false value

- **WHEN** a boolean-capable parameter supports and receives an explicit false value
- **THEN** the persisted selection SHALL distinguish it from inherited state
- **AND** the renderer SHALL apply the registry-declared false-value behavior

#### Scenario: Reject a mismatched typed value

- **WHEN** a submitted selection kind or value type does not match the registry control and renderer
- **THEN** validation SHALL reject the complete mutation with a structured field error

### Requirement: Canonical CLI capability registry

One canonical registry version SHALL define the user-editable CLI parameter contract. Native runtime behavior SHALL consume that registry directly, and any frontend static artifact required for Web/mock behavior SHALL be generated and contract-checked rather than independently maintained.

#### Scenario: Generate the Web contract

- **WHEN** the canonical registry changes
- **THEN** the repository generation task SHALL produce the corresponding typed frontend artifact deterministically
- **AND** repeated generation without source changes SHALL produce no diff

#### Scenario: Detect generated contract drift

- **WHEN** the generated frontend artifact does not match the canonical registry
- **THEN** the repository contract check SHALL fail
- **AND** production code SHALL NOT maintain a second hand-authored provider catalog as a fallback

#### Scenario: Web adapter renders a mock preview

- **WHEN** Web/mock mode previews a profile
- **THEN** it SHALL use the generated registry contract and shared deterministic TypeScript rendering semantics intended for mock behavior
- **AND** it SHALL identify the result as a non-launching Web preview

### Requirement: Version-aware parameter compatibility

The service SHALL evaluate each definition and explicit selection against cached active executable metadata, platform, launch scope, provider grammar, and registry compatibility rules. Compatibility evaluation SHALL be deterministic and SHALL NOT launch provider processes during page load, draft editing, or preview.

#### Scenario: Supported parameter and value

- **WHEN** the cached executable and selected scope satisfy a parameter's compatibility rules
- **THEN** the definition and value SHALL be marked supported
- **AND** the renderer MAY include its tokens after validation

#### Scenario: Installed version is too old

- **WHEN** a parameter requires a newer provider version than the cached active executable
- **THEN** the page SHALL mark the parameter unsupported with the required version
- **AND** runtime resolution SHALL omit an existing unsupported override and return a diagnostic

#### Scenario: Active version is unknown

- **WHEN** the CLI is available but its version is unknown or cannot be normalized
- **THEN** the service SHALL apply the registry's declared unknown-version policy
- **AND** the page SHALL distinguish unverified compatibility from confirmed support

#### Scenario: CLI is not installed

- **WHEN** the cached lifecycle state reports that the CLI is not installed
- **THEN** the user SHALL be able to inspect and edit the profile
- **AND** the page SHALL show that launch compatibility cannot be confirmed and link to CLI Management

#### Scenario: Compatibility changes after a CLI update

- **WHEN** cached lifecycle metadata changes so that a saved selection becomes unsupported
- **THEN** loading the profile SHALL preserve the selection for repair, mark it incompatible, and exclude it from launch rendering
- **AND** the system SHALL NOT silently replace it with a different provider value

### Requirement: Declarative dependencies and conflicts

The registry SHALL express bounded parameter dependencies, implications, and conflicts declaratively. The service SHALL evaluate those rules consistently for validation, UI state, preview, save, and runtime resolution.

#### Scenario: Required parameter is missing

- **WHEN** an explicit selection requires another parameter state that is not satisfied
- **THEN** draft preview SHALL return a field diagnostic
- **AND** profile save SHALL be rejected atomically

#### Scenario: Conflicting values are selected

- **WHEN** two explicit selections violate a declared conflict rule
- **THEN** both affected fields SHALL receive structured diagnostics
- **AND** no ambiguous token sequence SHALL be produced

#### Scenario: Special none list value conflicts with other values

- **WHEN** a list parameter declares `none` as exclusive and the draft also selects another value
- **THEN** the service SHALL reject or deterministically normalize the draft according to the declared registry rule
- **AND** the behavior SHALL be identical in desktop and Web/mock mode

### Requirement: Structured CLI parameter errors and diagnostics

All CLI parameter commands and application services SHALL return stable machine-readable error codes and diagnostics with typed context. Frontend behavior SHALL NOT depend on regular-expression parsing of localized or English backend error messages.

#### Scenario: Invalid field value

- **WHEN** validation rejects a submitted value
- **THEN** the error SHALL identify a stable code, agent id, parameter id, and localized-message key or structured details
- **AND** the page SHALL associate the error with the affected control

#### Scenario: Profile-level conflict

- **WHEN** a save fails because of revision, catalog, dependency, or compatibility conflict
- **THEN** the error SHALL expose a stable profile-level code and relevant expected and actual metadata
- **AND** no partial selection SHALL be persisted

#### Scenario: Diagnostic is safe to log

- **WHEN** a CLI parameter diagnostic is associated with an operation or written through unified logging
- **THEN** it SHALL exclude credentials, prompts, session identifiers, and unredacted secret-bearing environment values

### Requirement: Draft-safe preview service

The service boundary SHALL provide a read-only preview operation that accepts one managed agent id, one explicit launch scope, the baseline catalog version, and a complete in-memory selection map. Preview SHALL normalize and validate the draft without persisting it.

#### Scenario: Preview does not mutate persistence

- **WHEN** a valid or invalid draft preview is requested
- **THEN** the persisted profile, revision, and updated timestamp SHALL remain unchanged

#### Scenario: Preview returns field and profile diagnostics

- **WHEN** a draft contains invalid, incompatible, or conflicting selections
- **THEN** the preview response SHALL return structured diagnostics and only the safe token segments that remain valid under the registry contract
- **AND** profile save SHALL remain disabled while any blocking diagnostic exists

#### Scenario: Preview request is stale

- **WHEN** a slower preview response corresponds to an older draft or scope than the latest request
- **THEN** the frontend SHALL discard that response
- **AND** it SHALL not replace a newer preview or diagnostic set

### Requirement: CLI parameter page information architecture

The CLI Parameter Management page SHALL use an external-CLI navigation rail, active CLI header, scope and filter toolbar, grouped parameter editor, diagnostics area, safe argv preview, policy notice, and explicit profile actions. The page SHALL use existing compact settings primitives and avoid nested decorative cards.

#### Scenario: Show CLI operational state

- **WHEN** a managed CLI profile is selected
- **THEN** the header SHALL show the brand, cached active version, active executable path when available, and lifecycle status
- **AND** missing or conflicting states SHALL provide a link to CLI Management

#### Scenario: Show parameter source

- **WHEN** a field is rendered
- **THEN** the page SHALL state whether VaneHub inherits provider behavior or supplies a profile override
- **AND** it SHALL NOT claim to have resolved the provider's complete internal configuration when that data is unavailable

#### Scenario: Show policy ownership

- **WHEN** the page is loaded
- **THEN** a policy notice SHALL state that approval, automatic approval, permissions, sandboxing, and dangerous bypass behavior are not configured here
- **AND** it SHALL provide navigation to Agent Policies

#### Scenario: Use explicit profile actions

- **WHEN** a profile is dirty
- **THEN** the sticky action area SHALL provide Restore Inherited Values, Discard Draft, and Save Profile actions with appropriate disabled and loading states
- **AND** the page SHALL NOT imply that saving one profile atomically saves every other dirty profile

### Requirement: Curated provider registry v2 baseline

The first registry-v2 audit SHALL correct existing semantic drift and add only bounded, officially verified safe controls needed for the five managed CLIs. Registry options SHALL remain conservative when provider documentation is model-dependent, version-dependent, or incomplete.

#### Scenario: Claude Code registry baseline

- **WHEN** the Claude Code registry is verified
- **THEN** it SHALL support custom model identifiers, a verified model-dependent effort control, ordered fallback models, an inherited or explicit Chrome mode, setting-source selection, and verified accessibility or scripted-launch options
- **AND** version-gated options SHALL encode their documented minimum version
- **AND** prompts, session identity, output protocol, tool permission lists, approval modes, sandboxing, and dangerous bypass flags SHALL remain excluded

#### Scenario: Codex CLI registry baseline

- **WHEN** the Codex CLI registry is verified
- **THEN** model reasoning effort SHALL use the current stable configuration-reference values and a declarative configuration-value renderer
- **AND** any old value not supported by the accepted audit baseline SHALL be preserved only as a repair diagnostic rather than emitted
- **AND** approval and sandbox controls SHALL remain excluded

#### Scenario: Gemini CLI registry baseline

- **WHEN** the Gemini CLI registry is verified
- **THEN** it SHALL support verified model, diagnostic or accessibility controls, extension selection, and at most the documented maximum number of include directories
- **AND** exclusive list values and missing-directory warnings SHALL be enforced consistently
- **AND** prompts, sessions, output format, approval, allowed-tool, sandbox, and bypass behavior SHALL remain excluded or owned elsewhere

#### Scenario: OpenCode registry baseline

- **WHEN** the OpenCode registry is verified
- **THEN** model identifiers SHALL use provider/model guidance, model variant SHALL not be represented by one universal provider-independent enum, and thinking SHALL be described as displaying thinking blocks
- **AND** automatic approval and other policy-owned controls SHALL remain excluded

#### Scenario: Antigravity registry baseline

- **WHEN** the Antigravity registry is verified
- **THEN** only separately verified model, effort, and agent mappings SHALL be candidates for exposure
- **AND** Gemini CLI documentation SHALL NOT be treated as proof of an Antigravity CLI flag

### Requirement: Legacy profile repair workflow

The page SHALL expose non-destructive diagnostics and repair actions for migrated, unknown, malformed, deprecated, or incompatible persisted selections. Repair SHALL be explicit and scoped to the affected CLI profile.

#### Scenario: Display quarantined legacy value

- **WHEN** a profile contains a quarantined legacy row
- **THEN** the page SHALL identify the affected parameter or stored id, reason, and non-execution status
- **AND** valid unrelated selections SHALL remain editable and previewable

#### Scenario: Repair by selecting a supported value

- **WHEN** the user replaces a quarantined value with a valid current selection and saves with the current revision
- **THEN** the service SHALL persist the repaired profile in the current schema and remove the resolved diagnostic

#### Scenario: Repair by restoring inheritance

- **WHEN** the user restores inherited values for a quarantined profile and confirms the mutation
- **THEN** the obsolete rows SHALL be removed or rewritten according to the current schema
- **AND** the profile revision SHALL increment once

## REMOVED Requirements

### Requirement: All four managed CLIs use custom-text for model parameter

**Reason:** The requirement is internally inconsistent with the five managed external CLIs and with the existing OpenCode model control. It also embeds stale provider model lists and uses the literal string `default` as both an inheritance marker and a possible provider value.

**Migration:** Replace it with the modified typed-catalog requirements, explicit inherited selections, and the `Curated provider registry v2 baseline` requirement. Every exposed model control remains backend-authoritative, accepts bounded custom identifiers where verified, and receives current audited options rather than the removed hard-coded lists.
