## ADDED Requirements

### Requirement: Audited user-editable CLI parameter catalog
The user-editable CLI parameter catalog SHALL match the current supported launch arguments and meanings for Claude Code, Codex CLI, OpenCode, Antigravity CLI, and Gemini CLI, while policy-governed arguments remain managed only by Agent Policies.

#### Scenario: Compare frontend and native catalogs
- **WHEN** a managed CLI parameter profile is loaded in desktop or Web mode
- **THEN** both runtimes SHALL expose the same parameter ids, controls, launch scopes, defaults, flags, known values, and risk semantics

#### Scenario: Describe a managed parameter
- **WHEN** a managed parameter is displayed in any supported locale
- **THEN** its label and description SHALL state the effect of the actual emitted CLI argument
- **AND** known values SHALL reflect current supported aliases or choices without preventing a valid custom model value

#### Scenario: Keep policy controls single-sourced
- **WHEN** an argument controls approval, sandboxing, or another Agent policy
- **THEN** the CLI Parameters page SHALL omit that argument
- **AND** the effective argument preview SHALL continue to receive it from the Agent policy mapping when applicable

