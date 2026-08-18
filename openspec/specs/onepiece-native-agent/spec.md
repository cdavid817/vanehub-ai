# onepiece-native-agent Specification

## Purpose
TBD - created by archiving change add-onepiece-native-agent. Update Purpose after archive.
## Requirements
### Requirement: Stable built-in OnePiece identity
The system SHALL maintain OnePiece as a built-in Agent with stable id `onepiece`, display name `OnePiece`, `launch_kind = api`, `agent_origin = builtin`, API interaction support, and first-party capability metadata in both desktop and Web/mock registries.

#### Scenario: Initialize a clean registry
- **WHEN** VaneHub initializes a registry that has no `onepiece` row
- **THEN** the system SHALL seed the built-in OnePiece identity
- **AND** it SHALL keep the identity visible even though no provider configuration or credential exists yet

#### Scenario: Reopen an existing registry
- **WHEN** VaneHub starts against an existing database that does not contain OnePiece
- **THEN** idempotent registry seeding SHALL add OnePiece without modifying existing Agent, session, Skill, memory, usage, or Loop data

#### Scenario: Adopt an existing API Agent with the reserved id
- **WHEN** migration finds an existing `onepiece` row whose `launch_kind` is `api`
- **THEN** the system SHALL adopt that row as the built-in OnePiece identity in place
- **AND** it SHALL preserve its provider configuration, credential key, trust setting, sessions, messages, Skills, memories, usage records, and Loop references

#### Scenario: Reject an incompatible reserved-id collision
- **WHEN** migration finds an existing `onepiece` row whose launch kind is not `api`
- **THEN** initialization SHALL fail safely with a diagnostic identifying the incompatible reserved-id collision
- **AND** it SHALL NOT overwrite or delete the conflicting row

### Requirement: OnePiece configuration lifecycle
The system SHALL keep the OnePiece identity separate from multiple named endpoint Profiles, secure credentials independently per Profile, use at most one explicitly active Profile for direct generation, and expose provider-directory, custom endpoint, list, save, activate, delete, remove-all, privacy, capability, and routing operations through the shared frontend service boundary. Catalog Profiles SHALL resolve immutable reviewed endpoints; custom local/private Profiles SHALL accept only validated explicit OpenAI-compatible endpoint metadata and SHALL retain user-configured provenance.

#### Scenario: Browse supported providers
- **WHEN** a user starts adding an OnePiece configuration
- **THEN** the settings surface SHALL show the shared searchable versioned provider directory and a separate custom local/private endpoint action
- **AND** catalog endpoint records SHALL remain immutable and provenance-bearing

#### Scenario: Choose among provider endpoints
- **WHEN** a selected catalog provider exposes more than one endpoint type supported by the OnePiece runtime
- **THEN** the settings surface SHALL require or default an explicit endpoint selection and show the endpoint protocol and immutable Base URL
- **AND** saving SHALL persist the selected endpoint type with the Profile

#### Scenario: Reject an unavailable provider protocol
- **WHEN** a selected catalog provider does not publish the requested Anthropic or OpenAI endpoint, or the runtime does not support it
- **THEN** settings and application boundaries SHALL reject the selection without synthesizing a URL, storing a credential, or changing the active Profile

#### Scenario: Show unconfigured OnePiece
- **WHEN** OnePiece has no complete endpoint Profile
- **THEN** registry and settings surfaces SHALL still show OnePiece with an actionable non-selectable readiness state

#### Scenario: Add the first provider Profile
- **WHEN** a user selects a supported provider endpoint and supplies a Profile name, model id, and required credential while no OnePiece Profile exists
- **THEN** the system SHALL persist the non-secret Profile and automatically make it active
- **AND** it SHALL resolve catalog-owned fields from the directory and secure the credential through the existing credential boundary

#### Scenario: Add the first custom local Profile
- **WHEN** a user supplies a Profile name, validated Base URL, OpenAI-compatible interface, model id, authentication mode, timeout, privacy classification, capabilities, and context metadata while no Profile exists
- **THEN** the system SHALL persist the Profile with configured provenance and automatically make it active only when structurally ready
- **AND** optional absent authentication SHALL remain absent rather than becoming a placeholder key

#### Scenario: Add another provider Profile
- **WHEN** one or more OnePiece Profiles already exist and the user adds a catalog or custom endpoint
- **THEN** the new valid Profile SHALL be saved without replacing or activating over the current Profile

#### Scenario: Review multiple provider Profiles
- **WHEN** OnePiece has saved provider Profiles
- **THEN** the settings surface SHALL show Profile name, Local or Private label where justified, provider/runtime kind, interface, model, endpoint origin, authentication presence, timeout, privacy, capabilities, context provenance, readiness, and active state
- **AND** it SHALL NOT describe a Profile as secure merely because it is local

#### Scenario: Edit a provider Profile
- **WHEN** a user edits a saved OnePiece Profile
- **THEN** the system SHALL preserve its id and stored credential unless a replacement or authentication-mode change is submitted
- **AND** catalog-owned endpoint fields SHALL remain immutable while custom configured fields may be validated and changed
- **AND** editing an active Profile SHALL update future runtime snapshots without changing stable Agent id `onepiece`

#### Scenario: Reject an unknown or forged provider selection
- **WHEN** a caller provides provider, interface, or Base URL values outside a selected catalog endpoint contract
- **THEN** the application SHALL reject the request without storing credentials, contacting the submitted endpoint, or changing the active Profile

#### Scenario: Reject an unsafe custom endpoint
- **WHEN** a caller submits credentials in a URL, an unsupported scheme, an empty host, invalid timeout or context bounds, or inconsistent capabilities
- **THEN** the application SHALL reject the request before persistence or network contact

#### Scenario: Activate a provider Profile
- **WHEN** a user confirms activation of a structurally ready inactive Profile
- **THEN** the system SHALL make it the only active Profile for direct generation
- **AND** it SHALL preserve the selected Agent, current Session, and stable Agent id
- **AND** an in-flight generation SHALL retain its starting Profile snapshot

#### Scenario: Reject activation without a credential
- **WHEN** a Profile requires a credential that is absent or has incomplete structural configuration
- **THEN** activation SHALL fail without changing the current active Profile or contacting the provider

#### Scenario: Delete an inactive provider Profile
- **WHEN** a user confirms deletion of an inactive Profile
- **THEN** the system SHALL remove that Profile, its scoped credential, and routing references to it
- **AND** it SHALL leave the active runtime configuration unchanged

#### Scenario: Delete the active provider Profile
- **WHEN** a user confirms deletion of the active Profile
- **THEN** the system SHALL remove it and its scoped credential, clear direct active projection, disable rules that require it, and leave remaining Profiles inactive

#### Scenario: Use OnePiece provider setup on a narrow viewport
- **WHEN** the configuration and routing surfaces render at a narrow supported viewport
- **THEN** controls, Profile summaries, rule rows, status, and dialogs SHALL remain usable without horizontal page overflow

#### Scenario: Remove all OnePiece configuration
- **WHEN** a caller confirms the remove-all compatibility operation
- **THEN** the system SHALL delete all Profiles, credentials, and Hybrid rules, clear active runtime fields, and disable automatic tool approval
- **AND** it SHALL preserve OnePiece identity, sessions, Skill bindings, memories, usage, and Loop references

#### Scenario: Migrate a legacy single provider binding
- **WHEN** migration reads a catalog-backed or legacy OnePiece Profile created before Hybrid metadata existed
- **THEN** it SHALL preserve id, selection, provider fields, model, endpoint, credential mapping, and active state
- **AND** it SHALL assign conservative compatibility defaults without classifying an arbitrary endpoint as local or verified

### Requirement: OnePiece provider model discovery
The system SHALL discover selectable OnePiece chat models through a shared service contract using only the selected catalog endpoint's model-list URL and authentication strategy, SHALL provide reviewed catalog models when live discovery is unavailable, and SHALL NOT require or permit free-text model identifiers in the OnePiece configuration UI.

#### Scenario: Discover models for a new Profile
- **WHEN** a user selects a catalog provider endpoint, enters a new API credential, and requests its models
- **THEN** the desktop adapter SHALL submit the provider id, endpoint type, and transient credential through the service boundary
- **AND** the native runtime SHALL contact only that endpoint record's compiled HTTPS model-list URL with its declared authentication strategy
- **AND** the raw credential, request headers, and provider response body SHALL NOT be persisted, returned, or written to logs

#### Scenario: Refresh models for an existing Profile
- **WHEN** a user requests models for an existing Profile without entering a replacement credential
- **THEN** the native runtime SHALL use that Profile's scoped stored credential
- **AND** it SHALL NOT reveal the stored credential to the frontend

#### Scenario: Normalize discovered models
- **WHEN** a supported provider returns a model list
- **THEN** the system SHALL trim and deduplicate identifiers, exclude known non-chat model types, merge reviewed catalog fallback models, and return deterministic searchable model options
- **AND** it SHALL NOT infer unsupported OnePiece tool or reasoning capabilities solely from an unknown model identifier

#### Scenario: Degrade when discovery fails
- **WHEN** a provider model-list request times out, is rejected, or returns an invalid response
- **THEN** the service SHALL return the preset's reviewed fallback models with a safe warning when fallback models exist
- **AND** unified logs SHALL contain only safe provider id, duration/count, and error-category metadata

#### Scenario: Preserve a historical selection
- **WHEN** an existing Profile's model is absent from the current live and fallback lists
- **THEN** the model selector SHALL retain and label the historical value instead of silently replacing it

#### Scenario: Discover models in Web/mock mode
- **WHEN** the Web/mock adapter receives a model-discovery request with valid mock input
- **THEN** it SHALL return deterministic models from the shared catalog contract without contacting a provider or retaining the submitted credential

### Requirement: OnePiece API-key verification
The system SHALL expose the shared API-credential verification capability for every catalog-backed OnePiece Profile using only the selected provider directory endpoint, selected model, and either a transient replacement credential or the Profile-scoped stored credential.

#### Scenario: Verify a new OnePiece credential
- **WHEN** a user selects a supported OnePiece provider endpoint and model, enters an API key in the add dialog, and requests verification
- **THEN** the frontend SHALL submit the provider id, endpoint type, model id, and transient credential through `AgentService`
- **AND** the native application SHALL resolve the immutable endpoint from the compiled directory and SHALL NOT persist the transient credential

#### Scenario: Verify an edited OnePiece Profile
- **WHEN** a user requests verification while editing an existing OnePiece Profile
- **THEN** a non-empty transient replacement credential SHALL take precedence for that check
- **AND** when no replacement is entered the native application SHALL use the Profile-scoped stored credential without revealing it to the frontend

#### Scenario: Verify from a OnePiece Profile card
- **WHEN** a saved OnePiece Profile with a configured credential is displayed in Agent configuration
- **THEN** its card SHALL provide a verification action that uses the saved provider endpoint, model, and scoped credential
- **AND** the result SHALL be displayed as ephemeral Profile feedback without activating the Profile or changing runtime configuration

#### Scenario: Reject a forged OnePiece verification target
- **WHEN** a caller requests verification with an unknown provider id, an endpoint type not owned by that provider, a mismatched Profile id, or a model that fails the existing model-id rules
- **THEN** the application SHALL reject the request before provider contact
- **AND** it SHALL NOT load or store a credential for the forged target

### Requirement: Versioned OnePiece core instructions
The system SHALL apply non-removable, versioned OnePiece core instructions to every OnePiece generation before optional Skill and memory sections, and the shipped core content MUST NOT exceed 8,000 Unicode characters.

#### Scenario: Generate without Skills or memories
- **WHEN** OnePiece starts a generation with no bound Skills and no scoped memories
- **THEN** the provider request SHALL still contain the complete OnePiece core-instruction section

#### Scenario: Trace the core version
- **WHEN** an OnePiece generation resolves its core instructions
- **THEN** safe generation diagnostics SHALL record the stable core-instruction version
- **AND** diagnostics SHALL NOT record the full core content

#### Scenario: Core asset exceeds its budget
- **WHEN** verification inspects a shipped OnePiece core-instruction asset longer than 8,000 Unicode characters
- **THEN** verification SHALL fail rather than truncating or shipping a partial identity prompt

#### Scenario: User manages Skills
- **WHEN** a user disables, unbinds, or deletes a Skill associated with OnePiece
- **THEN** the operation SHALL affect only the Skill section
- **AND** it SHALL NOT remove, edit, or disable OnePiece core instructions

### Requirement: Safe OnePiece tool defaults
The system SHALL initialize and reset OnePiece with automatic shell, file-write, and file-edit approval disabled and SHALL continue applying the existing MCP approval and plan-mode restrictions. Read-only content-search and filename-search calls SHALL NOT require approval.

#### Scenario: First configuration retains approval prompts
- **WHEN** OnePiece is configured for the first time
- **THEN** shell, file-write, and file-edit calls SHALL require approval until the user explicitly enables the existing trust setting

#### Scenario: Read-only search does not prompt
- **WHEN** OnePiece requests a content-search or filename-search tool call
- **THEN** the system SHALL execute it without an approval prompt regardless of the trust setting

#### Scenario: Trust does not bypass existing hard gates
- **WHEN** a trusted OnePiece requests an MCP tool or runs in plan mode
- **THEN** the existing MCP approval and plan-mode restrictions SHALL remain in force

### Requirement: OnePiece runtime and presentation parity
Desktop and Web/mock runtimes SHALL expose OnePiece through the same frontend service contracts, stable visual identity, configuration states, and session-selection behavior while differing only in native persistence, credential, and provider execution.

#### Scenario: Place OnePiece after CLI configuration tabs
- **WHEN** the Agent configuration page renders the Claude Code, OpenCode, Codex CLI, and OnePiece tabs
- **THEN** it SHALL place the three CLI tabs before OnePiece
- **AND** OnePiece SHALL be the final tab while remaining directly navigable

#### Scenario: Render OnePiece identity
- **WHEN** registry, settings, create-session, session-list, or session-detail UI renders stable id `onepiece`
- **THEN** it SHALL render the OnePiece display identity without persisting redundant icon metadata on sessions

#### Scenario: Configure OnePiece in Web/mock mode
- **WHEN** a user configures OnePiece in Web/mock mode with valid mock input
- **THEN** the Web adapter SHALL transition its in-memory readiness through the shared service contract
- **AND** it SHALL NOT access SQLite, the OS credential store, or a real provider

#### Scenario: Generate through the desktop runtime
- **WHEN** a ready OnePiece receives a message in the desktop runtime
- **THEN** the existing API process gateway SHALL execute the generation without a OnePiece-specific provider execution branch

### Requirement: OnePiece fine-grained Token accounting
OnePiece SHALL account for every provider request through the shared API-Agent invocation accounting contract and SHALL preserve Profile, endpoint, provider, and model attribution captured at invocation start.

#### Scenario: Switch Profile after generation starts
- **WHEN** the active OnePiece Profile changes while a generation is running
- **THEN** usage from the running generation SHALL remain attributed to its immutable starting Profile snapshot
- **AND** later requests SHALL use the newly active Profile only when their generation starts

#### Scenario: Compare OnePiece consumption purposes
- **WHEN** OnePiece performs visible response, tool-continuation, compaction, or memory-extraction calls
- **THEN** usage consumers SHALL be able to distinguish each purpose while also viewing total OnePiece consumption

#### Scenario: OnePiece provider omits usage
- **WHEN** an otherwise successful OnePiece request completes without valid provider usage
- **THEN** the runtime SHALL expose reduced reported coverage and apply only the permitted estimation fallback
- **AND** OnePiece SHALL remain usable

#### Scenario: Preserve Web/mock parity
- **WHEN** OnePiece runs in Web/mock mode
- **THEN** the adapter SHALL expose deterministic invocation, purpose, provider, model, and quality fixtures through the shared service contract
- **AND** it SHALL NOT contact a provider

### Requirement: Bounded OnePiece planning requests
The OnePiece native runtime SHALL support a versioned, tool-less planning request that uses the active Profile, receives a bounded structured schema and execution-tool descriptions, and returns content for strict task-orchestration validation without creating execution worktrees or Worker sessions.

#### Scenario: Invoke the planner with an active Profile
- **WHEN** task orchestration requests a Plan draft and OnePiece has an active ready Profile
- **THEN** the runtime SHALL capture that Profile's provider configuration for the generation and SHALL execute no tools during the planning request

#### Scenario: Reject planning without readiness
- **WHEN** task orchestration requests planning but no active OnePiece Profile is ready
- **THEN** the runtime SHALL return an actionable readiness error without starting provider generation or mutating Plan execution state

### Requirement: Attempt execution profile
The OnePiece native runtime SHALL accept an attempt-scoped execution profile containing a bounded root, versioned task instructions, permitted tool catalog, tool-call limit, token budget, and timeout, and SHALL correlate the resulting generation with the supplied PlanRun, SubTaskRun, and Attempt identities.

#### Scenario: Start an attempt generation
- **WHEN** task orchestration starts a valid SubTask attempt
- **THEN** OnePiece SHALL execute through the existing API process gateway using the captured active Profile and the attempt's bounded workspace and limits

#### Scenario: Enforce an attempt limit
- **WHEN** an attempt reaches a configured tool-call, token, or timeout boundary
- **THEN** the runtime SHALL stop at the nearest safe execution boundary and return a classified limit outcome to task orchestration

### Requirement: OnePiece credential reference isolation
Planner and SubTask execution SHALL reuse Profile-scoped OnePiece credentials through the existing credential boundary and SHALL NOT copy credential values into Plan records, task prompts, Agent session metadata, operation metadata, or execution telemetry.

#### Scenario: Persist orchestration metadata
- **WHEN** the runtime stores a planner call or SubTask attempt
- **THEN** it SHALL retain only safe Profile and generation references needed for audit and SHALL keep the credential in its existing secure store

### Requirement: OnePiece project discovery execution profile
The native OnePiece runtime SHALL support a planning discovery profile that is bound to one canonical local project, advertises only the approved read-only discovery tools, applies independent tool, token, context, and time limits, and returns structured Plan output through the captured active Profile without copying credentials.

#### Scenario: Run bounded discovery
- **WHEN** task orchestration requests project-aware planning with a ready captured OnePiece Profile
- **THEN** OnePiece SHALL perform only allowed workspace-scoped discovery and return the requested strict Plan structure with discovery limitation metadata

#### Scenario: Model requests a prohibited planning tool
- **WHEN** OnePiece requests shell, file mutation, MCP, memory mutation, arbitrary network, or an operation outside the canonical project during discovery
- **THEN** the runtime SHALL reject the call regardless of model output and SHALL preserve an actionable planning failure

### Requirement: OnePiece repair execution profile
The native OnePiece runtime SHALL support a repair profile that starts a distinct attempt session in the retained PlanRun worktree and receives only the current SubTask or final-repair instructions, acceptance criteria, bounded prior failure evidence, current changed-file summary, and snapshotted limits.

#### Scenario: Start a repair Attempt
- **WHEN** task orchestration dispatches an eligible repair
- **THEN** OnePiece SHALL receive bounded failed-check evidence without raw predecessor transcripts, credentials, unbounded command output, or unrelated historical attempts

#### Scenario: Repair reaches a limit
- **WHEN** the repair session reaches its tool, token, or timeout limit
- **THEN** OnePiece SHALL stop through the existing safe limit boundary and the attempt SHALL retain a classified terminal outcome

### Requirement: OnePiece extended tool capability and readiness
The system SHALL project the Browser, Web research, code-execution, OCR, Artifact-publication, and CLI-delegation capabilities only on the built-in OnePiece identity and SHALL expose mode-specific readiness and safe reason codes without making OnePiece chat readiness depend on every optional tool. User-created API Agents SHALL not inherit these capabilities from provider configuration or capability-tag editing.

#### Scenario: Optional dependency is unavailable
- **WHEN** OnePiece's provider Profile is ready but an optional browser, OCR, sandbox, Artifact, or delegated-CLI dependency is unavailable
- **THEN** OnePiece SHALL remain available for ordinary chat and baseline tools while the affected extended operation is excluded or reported unavailable

#### Scenario: OnePiece has one usable delegated target
- **WHEN** at least one supported target/mode passes delegation readiness
- **THEN** OnePiece MAY receive the fixed `delegate_cli` definition while unavailable targets remain dispatch-time errors with actionable reasons

#### Scenario: Custom API Agent copies capability metadata
- **WHEN** a user-created API Agent has metadata resembling OnePiece
- **THEN** the native tool registry SHALL still deny eligibility because its stable id is not `onepiece`

### Requirement: Safe defaults for extended effects
OnePiece SHALL default to explicit unified approval for arbitrary code execution, effectful browser actions, retained downloads, external CLI delegation start, and delegated ChangeSet application. ChangeSet application approval SHALL always be once-only and SHALL not become automatically allowed through session, project, global, trusted, or YOLO-style remembered scopes.

#### Scenario: First use of code execution
- **WHEN** a newly configured OnePiece requests `code_execution`
- **THEN** the system SHALL request approval bound to the exact source, runtime, inputs, and limits unless a non-remembered explicit policy decision for that call already exists

#### Scenario: User previously trusted ordinary OnePiece tools
- **WHEN** OnePiece's existing policy allows shell or file writes automatically
- **THEN** `apply_delegation_changes` SHALL still require its specialized once-only exact-ChangeSet approval

### Requirement: Extended readiness is available through shared adapters
The frontend SHALL obtain OnePiece extended-capability readiness and operation state through shared service contracts with Tauri and Web/mock implementations. The Web/mock registry SHALL preserve the same OnePiece identity and capability presentation while identifying native execution as simulated or desktop-required.

#### Scenario: Desktop settings inspect readiness
- **WHEN** a user opens OnePiece capability diagnostics
- **THEN** the Tauri adapter SHALL return native per-capability and per-mode readiness without starting a browser, OCR inference, sandbox program, or delegated model call

#### Scenario: Web settings inspect readiness
- **WHEN** the same surface runs in Web/mock mode
- **THEN** the Web adapter SHALL return deterministic non-native readiness without implying installed desktop dependencies

### Requirement: OnePiece assembles proactive evidence through the Context Engine
For eligible project turns, OnePiece SHALL invoke the Context Engine before final provider request construction and SHALL accept only a verified bounded projection; optional candidate-source failure or manifest persistence failure MUST NOT fail the generation.

#### Scenario: Evidence selection succeeds
- **WHEN** the Context Engine returns a verified evidence set
- **THEN** OnePiece SHALL include its compact projection in the provider request
- **AND** it SHALL preserve existing provider, tool, cancellation, accounting, and compaction behavior

#### Scenario: Engine cannot produce a safe projection
- **WHEN** planning, collection, normalization, ranking, budgeting, or verification cannot safely complete
- **THEN** OnePiece SHALL continue through the existing request path without partial injected evidence
- **AND** it SHALL emit only a bounded redacted outcome

### Requirement: OnePiece Profile operations preserve adapter parity
Desktop and Web/mock adapters SHALL expose contract-compatible Profile, discovery, verification, capability, privacy, and routing states; Web/mock SHALL simulate them without SQLite, credential-store, or network access.

#### Scenario: Configure Hybrid Routing in Web mode
- **WHEN** a user creates Profiles and rules through the Web/mock settings surface
- **THEN** deterministic in-memory state SHALL produce the same frontend contract and validation semantics as desktop

### Requirement: OnePiece uses routed Profiles through the shared API gateway
OnePiece SHALL execute a selected local, private, or cloud Profile through the existing API generation gateway and MUST NOT add endpoint-product-specific generation branches.

#### Scenario: Complete a local text turn
- **WHEN** routing selects a ready OpenAI-compatible localhost Profile that supports text generation
- **THEN** OnePiece SHALL stream the turn through the shared API request/event path
- **AND** endpoint-product identity SHALL not alter generic request orchestration

