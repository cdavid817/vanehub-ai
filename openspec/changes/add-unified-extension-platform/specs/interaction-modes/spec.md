## ADDED Requirements

### Requirement: Extensions may contribute declarative mode presets

An enabled extension MAY contribute a namespaced data-only mode preset referencing an existing registered execution strategy, optional policy template, tool groups, Skills, Hooks, and configuration schema. The preset SHALL be validated without executing extension code and SHALL be ineligible when required references are unavailable.

#### Scenario: Guarded preset is valid

* WHEN a preset references the registered guardrails strategy, an existing policy template, and eligible Hooks/tools
* THEN it appears as an extension-sourced selectable preset with contribution provenance

#### Scenario: Preset references unknown strategy

* WHEN the strategy id is not in the application registry
* THEN the preset is ineligible and cannot be selected

### Requirement: Executable third-party mode strategies are prohibited

Version 1 external extensions SHALL NOT register arbitrary scheduler, planner, supervisor, router, or executor code as a new interaction strategy. Only reviewed built-in code may add a strategy implementation.

#### Scenario: Manifest declares executable mode handler

* WHEN an external mode contribution includes a runtime entrypoint as its strategy implementation
* THEN contribution validation rejects it

### Requirement: Mode presets cannot weaken safety floors

Selecting an extension mode preset SHALL not reduce immutable permission floors, bypass approval, disable required safety Hooks, broaden tool visibility beyond eligibility, or grant missing connector credentials.

#### Scenario: Preset requests permissive policy

* WHEN a preset references a permissive policy template but a rule/floor requires Ask or Deny
* THEN the stronger decision remains effective
