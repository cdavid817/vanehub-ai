## ADDED Requirements

### Requirement: Skill tool management panel
The Skill detail surface SHALL provide a Tools panel that lists canonical tool ids, implementation kind, effective revision, requested capabilities, integrity, trust, enablement, validation, quarantine, and latest bounded runtime status. The panel SHALL obtain data and mutations through the frontend Skill service boundary.

#### Scenario: User inspects Skill tools
- **WHEN** a Skill with a tool manifest is selected
- **THEN** the Tools panel displays its effective tool inventory and clearly distinguishes trusted, disabled, invalid, and quarantined states

#### Scenario: React submits a tool mutation
- **WHEN** a user trusts, enables, disables, validates, quarantines, or recovers a Skill tool revision
- **THEN** the component calls the frontend service interface and does not invoke the native runtime directly

### Requirement: Explicit revision trust flow
The UI SHALL show the exact revision and integrity witness, capability diff, validation result, and source scope before accepting a trust decision. Content or capability changes SHALL require a new decision and MUST NOT reuse the previous confirmation.

#### Scenario: User reviews an updated module
- **WHEN** a previously trusted Skill tool has a changed manifest, module hash, or capability set
- **THEN** the UI marks it untrusted and presents the revision changes before trust can be granted

### Requirement: Honest desktop and Web behavior
The Tauri adapter SHALL support native inspection and execution-management operations. The Web adapter SHALL return an explicit unsupported execution capability while preserving inspectable mock or remote-ready contracts and MUST NOT report local module execution as successful.

#### Scenario: Web runtime opens the Tools panel
- **WHEN** the page runs without a native or configured remote Skill tool backend
- **THEN** the panel remains usable for inspection but disables native execution actions with an unsupported explanation

### Requirement: Accessible runtime diagnostics
The panel SHALL expose redacted validation failures, limit breaches, quarantine causes, recent invocation outcomes, and recovery actions with keyboard-accessible controls and status text that does not rely on color alone.

#### Scenario: Tool is quarantined after failures
- **WHEN** a selected tool revision is quarantined
- **THEN** the panel identifies the reason, affected revision, latest redacted failures, and available recovery action

