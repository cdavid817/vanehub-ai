## ADDED Requirements

### Requirement: Effective Skill tool metadata
The effective Skill model SHALL expose whether its winning revision contains a tool manifest, the manifest and package integrity state, tool count, trust state, enablement state, validation state, and quarantine summary without loading executable content into the frontend.

#### Scenario: Effective Skill includes tools
- **WHEN** Skill details are requested for a winning revision with a tool manifest
- **THEN** the response includes bounded tool metadata and integrity witnesses for that exact revision

#### Scenario: Lower-priority revision includes tools
- **WHEN** a shadowed Skill revision contains tools but the winning revision does not
- **THEN** the shadowed tools are not reported as active or available

### Requirement: Tool lifecycle follows Skill lifecycle
Disabling, archiving, deleting, replacing, restoring, or changing the winning revision of a Skill SHALL trigger an atomic refresh of that Skill's contributed tools. A pinned Skill remains protected from content mutation but MAY have its tool execution disabled as an independent safety control.

#### Scenario: Skill is disabled
- **WHEN** a Skill with registered tools is disabled
- **THEN** new invocations of those tools become unavailable after the atomic registry refresh

#### Scenario: Pinned Skill tool is disabled
- **WHEN** an authorized user disables tool execution for a pinned Skill without changing its content
- **THEN** the system disables new tool invocations while preserving the pinned Skill revision

