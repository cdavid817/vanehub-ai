## ADDED Requirements

### Requirement: Settings provides one unified Extensions workspace

The frontend SHALL provide Settings → Extensions with Installed, Contributions, Hooks, Rules, Connections, and Diagnostics tabs behind the frontend service boundary. The page SHALL NOT call Tauri directly and SHALL use matching Tauri and Web/mock service contracts.

#### Scenario: User opens Extensions settings

* WHEN the Extensions route loads
* THEN the default Installed tab shows service-provided state and the remaining tabs are directly addressable by stable query parameter

#### Scenario: Application runs in Web/mock mode

* WHEN the same route loads in Web/mock
* THEN deterministic fixture data and honest unsupported-native messaging are shown without claiming package extraction, credential access, process launch, or persistent native effects

### Requirement: Installed extensions are searchable and inspectable

The Installed tab SHALL support search and filters for state, source, runtime, trust profile, signature/publisher state, and contribution kind. Each item SHALL show stable identity, publisher, version, status, runtime/trust, signature, contribution counts, health, and available lifecycle actions.

#### Scenario: User filters quarantined WASM extensions

* WHEN state Quarantined and runtime WASM are selected
* THEN only matching items are displayed and filter state is keyboard accessible and restorable within the page session

#### Scenario: Lifecycle action is unavailable

* WHEN an item cannot be reloaded because it is incompatible or an operation is already running
* THEN the action is disabled with an accessible explanation rather than failing after an avoidable click

### Requirement: Local package installation uses a staged security review

The UI SHALL implement a staged `.vhext` install/update wizard that shows package validation, publisher/signature, compatibility, dependencies, contributions, requested capabilities, authority diff, permitted trust profiles, immutable preview witness, operation progress, and final enablement choice. The UI SHALL distinguish provenance, runtime capabilities, Agent operation permissions, connector credentials, and Skill Tool trust.

#### Scenario: Signed update expands network authority

* WHEN an update preview adds a network origin
* THEN the wizard highlights the expansion and requires a fresh confirmation before the operation starts

#### Scenario: Unsigned package is selected

* WHEN Developer Mode is disabled
* THEN the wizard blocks installation and explains the signature requirement; WHEN Developer Mode is enabled, it identifies Strict/disabled containment and persistent risk warning

### Requirement: Extension details expose contribution and runtime provenance

The UI SHALL provide Overview, Contributions, Permissions, Dependencies, Runtime, and Logs views for an extension. It SHALL show extension id/version/hash, snapshot/runtime/registry generation, contribution global ids, eligibility reasons, capability state, dependency resolution, activation events, health, operation history, and redacted logs.

#### Scenario: User inspects an ineligible tool

* WHEN a tool is unavailable because a Skill dependency is missing
* THEN details identify the exact contribution, dependency constraint, and recovery action without activating the runtime

#### Scenario: User inspects sidecar runtime

* WHEN a Trusted sidecar is active
* THEN the UI may show safe process/runtime health metadata but never full environment, command-line secrets, raw credential values, or unrestricted filesystem paths

### Requirement: Contributions explorer links to authoritative domains

The Contributions tab SHALL group tools, Skills, MCP definitions, modes, Hooks, rules, and connectors and SHALL show source extension, global id, eligibility, runtime/dependency state, and domain-specific summary. Where a specialist settings page remains authoritative, the UI SHALL provide a deep link rather than duplicating all edit behavior.

#### Scenario: User selects an extension-contributed Skill

* WHEN the Skill is selected
* THEN the explorer shows immutable extension provenance and links to Skill details/configuration while preventing base-content editing in the unified page

#### Scenario: User selects an MCP definition

* WHEN an extension-owned MCP definition is selected
* THEN the explorer links to MCP credential/binding configuration and indicates that secrets are not packaged with the extension

### Requirement: Hooks UI is event-aware and traceable

The Hooks tab SHALL list Hook id/name, event, handler kind, source/scope, priority, state, failure mode, recent latency/error, and circuit state. It SHALL support permitted create/edit/duplicate/enable/disable/delete actions, synthetic testing, execution trace, and Claude compatibility import preview. Event-specific editors SHALL expose only admissible matchers and decisions.

#### Scenario: User edits a before-tool Hook

* WHEN `tool.before_execute` is selected
* THEN the editor offers applicable tool/operation/risk matchers and Deny/Ask/allowed input-patch behavior but not invalid after-tool output decisions

#### Scenario: Extension Hook is immutable

* WHEN the selected Hook originates from an extension manifest
* THEN its definition is read-only and the UI offers extension disable or user-scope duplication only where supported

### Requirement: Rules UI separates templates from operation-specific rules

The Rules tab SHALL show source, operation, effect, risk, priority, scope, expiry, state, and immutability; provide a structured operation-aware editor and YAML preview; show project-file and last-known-good diagnostics; and link to Agent Policies to explain template fallback.

#### Scenario: User edits a project rule

* WHEN preview detects that the rule broadens an Allow match or approval scope
* THEN the UI displays the authority expansion before confirmation

#### Scenario: Project YAML is invalid

* WHEN reload fails
* THEN the UI states that the last-known-good generation remains active and identifies the safe source/rule/field error

### Requirement: Rule simulation explains the complete decision chain

The UI SHALL provide a non-executing decision simulator with principal, Agent/session/project, operation type, resource/arguments, and risk inputs. Results SHALL show normalization, safety floors, matching rules, source/priority/specificity, template fallback, Hook strengthening policy, grant eligibility, eligible approval scopes, generation, and final simulated decision.

#### Scenario: User simulates a critical command

* WHEN a critical destructive shell command is simulated
* THEN the UI clearly shows the floor/rules that require Deny or Once approval and does not run the command

#### Scenario: No rule matches

* WHEN simulation falls back to the Agent's policy template
* THEN the UI labels the template as fallback instead of presenting it as a matched rule

### Requirement: Connections UI unifies lifecycle without hiding source ownership

The Connections tab SHALL show connector name/type/source, configuration/auth/connection/health, capabilities, bindings, last test, redacted error, and supported actions. Projected GitHub, IM, and MCP connectors SHALL identify their authoritative subsystem and deep-link when advanced configuration remains elsewhere.

#### Scenario: User authenticates GitHub CLI connector

* WHEN the connector uses external-CLI authentication
* THEN the UI explains the required CLI flow and refreshes readiness through a stable operation without requesting a raw token

#### Scenario: User reconnects an IM connector

* WHEN reconnect is initiated from the unified page
* THEN progress reflects the delegated Communications operation and stale UI completion cannot overwrite a later state

### Requirement: Diagnostics are actionable and redacted

The Diagnostics tab SHALL expose package validation/quarantine, lifecycle operations, registry/runtime generations, activation duration, crashes/timeouts/circuit breakers, contribution adapter rollback, Hook statistics, rule compilation, connector health/auth expiry, and copyable redacted reports. It SHALL NOT reveal raw credentials, authorization headers, unrestricted prompt/tool payloads, full environment variables, or sensitive user paths.

#### Scenario: Extension is quarantined

* WHEN a crash loop quarantines an extension
* THEN diagnostics show threshold evidence, last failures, active/known-good versions, and reset/rollback/disable/uninstall actions

#### Scenario: User copies diagnostic report

* WHEN copy report is invoked
* THEN the generated report applies the same native redaction policy and labels omitted values

### Requirement: Legacy routes remain compatible

The existing Plugin Integrations route SHALL redirect to the Connections tab while retaining enough state to locate the corresponding built-in connector. Existing specialist routes for local extensions, Skills, MCP, Prompt Hooks, Agent Policies, and IM connectors SHALL remain reachable for at least one release.

#### Scenario: Old GitHub integration deep link is opened

* WHEN a legacy route/query identifies GitHub integration
* THEN the application opens Connections with the GitHub connector selected or highlighted

#### Scenario: Specialist page is needed

* WHEN a unified row requires advanced Skill, MCP, Prompt Hook, policy, local-extension, or IM settings
* THEN the user can navigate to the authoritative page and return without losing the unified tab context

### Requirement: UI meets repository quality, accessibility, and localization constraints

All production TS/TSX files for the workspace SHALL remain at or below 300 physical lines, use React function components and existing service/state/style conventions, semantic design tokens and Tailwind, and provide keyboard navigation, visible focus, accessible names/status announcements, compact desktop density, responsive stacking, loading/empty/error/disabled states, and translations for all supported locales.

#### Scenario: Keyboard-only install flow

* WHEN a user operates the install wizard without a pointer
* THEN every step, permission group, confirmation, cancel action, and error summary is reachable with logical focus order

#### Scenario: Locale changes

* WHEN the application switches to any supported locale
* THEN the new workspace renders translated labels/messages without hard-coded fallback text or layout-breaking missing keys
