## ADDED Requirements

### Requirement: Extension tool catalog admission is bounded and collision-free

Extension-contributed tools SHALL enter the provider catalog alongside the fixed handler registry and MCP-sourced entries, never replacing or shadowing a fixed tool name. Admission SHALL be bounded by an explicit aggregate ceiling per generation using stable extension-id and contribution-id ordering, and overflow SHALL preserve every fixed tool and record one bounded warning. Changes to browser, OCR, sandbox, or delegated-CLI runtime inventory SHALL continue to leave fixed handler schemas unchanged; extension contributions SHALL NOT be treated as such a runtime-inventory change.

#### Scenario: Extension tool shares a fixed tool name

* WHEN an extension declares a contribution whose display name matches `shell`, `file`, or another fixed catalog name
* THEN its catalog entry remains distinguishable by namespaced global id and SHALL NOT replace or shadow the fixed tool

#### Scenario: Aggregate extension catalog exceeds its ceiling

* WHEN eligible extension tools for one generation exceed the configured aggregate ceiling
* THEN admission is truncated in stable order, every fixed tool is preserved, and one bounded overflow warning is recorded

### Requirement: Extension tools use the native tool execution lifecycle

Eligible extension tools SHALL register through the native tool catalog with namespaced identity, extension/snapshot/runtime provenance, input/output schema, activation event, handler kind, and requested capabilities. Invocation SHALL use the same schema validation, call-time eligibility, before/after Hooks, Permissions/approval, timeout, cancellation, output limits, tracing, and audit as other native tools.

#### Scenario: Cold extension tool is invoked

* WHEN an eligible tool's runtime is inactive
* THEN tool execution performs single-flight lazy activation before permissioned handler invocation

#### Scenario: Tool becomes disabled after model selection

* WHEN the extension or contribution is disabled before call-time validation
* THEN execution rejects the stale tool call and does not activate or invoke the handler

### Requirement: Tool execution pins extension generation

Each extension tool call SHALL pin the contribution-registry and runtime generation on which it began. Reload, rollback, disable, or uninstall SHALL affect new calls and SHALL drain/cancel old calls according to extension lifecycle policy.

#### Scenario: Reload occurs during extension tool execution

* WHEN the call is in flight during a successful reload
* THEN it completes or is cancelled on the old generation and its result retains old-generation provenance

### Requirement: External CLI tool injection is not implied

Version 1 SHALL expose extension tools to the OnePiece/native Agent. It SHALL NOT claim that tools are available to managed external CLI Agents unless a separate existing integration explicitly supports a safe tool bridge and has matching specs/tests.

#### Scenario: User selects a CLI Agent

* WHEN an extension tool has no supported CLI bridge
* THEN it is not advertised as callable by that CLI Agent and UI explains the interface limitation
