# native-model-discovery Specification

## Purpose
TBD - created by archiving change dynamic-llm-model-discovery. Update Purpose after archive.
## Requirements
### Requirement: Discover model from Claude Code native config
The system SHALL read the active model from Claude Code's `settings.json` at `~/.claude/settings.json` when building session chat configuration defaults for the `claude-code` agent.

#### Scenario: Model found in settings.json env block
- **WHEN** `~/.claude/settings.json` exists and contains `{"env": {"ANTHROPIC_MODEL": "claude-sonnet-5"}}`
- **THEN** the discovered model ID `claude-sonnet-5` SHALL be used as the session's initial model

#### Scenario: Model missing from env block
- **WHEN** `~/.claude/settings.json` exists but does not contain `env.ANTHROPIC_MODEL`
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `claude-code`

#### Scenario: Config file absent
- **WHEN** `~/.claude/settings.json` does not exist
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `claude-code`

#### Scenario: Config file is malformed JSON
- **WHEN** `~/.claude/settings.json` exists but contains invalid JSON
- **THEN** the system SHALL log a diagnostic warning and fall back to the VaneHub CLI profile default model

### Requirement: Fall back to Claude Code's per-project usage cache
When `~/.claude/settings.json` does not yield a model, the system SHALL attempt to discover the active model from Claude Code's per-project state file at `~/.claude.json`, keyed by the session's workspace path under `projects[path].lastModelUsage`. This source SHALL only be trusted when it names exactly one model.

#### Scenario: Project cache has a single recorded model
- **WHEN** `~/.claude/settings.json` has no `ANTHROPIC_MODEL` and `~/.claude.json` contains a `projects` entry for the session's workspace path with `lastModelUsage` naming exactly one model
- **THEN** that model ID SHALL be used as the session's initial model

#### Scenario: Project cache has usage for multiple models
- **WHEN** the matched project's `lastModelUsage` names two or more models
- **THEN** the system SHALL treat the result as unknown and fall back to the VaneHub CLI profile default model, since no field indicates which model was most recently active

#### Scenario: Workspace path uses Windows-style separators
- **WHEN** the session's workspace path uses backslash separators (e.g. `D:\project\path`) and the matching `~/.claude.json` project key uses forward slashes (e.g. `D:/project/path`)
- **THEN** the system SHALL normalize both paths (case-insensitive, forward slashes, no trailing slash) before comparing, and SHALL still match

#### Scenario: No workspace path available
- **WHEN** the session has no workspace path (neither worktree path nor project path)
- **THEN** the system SHALL skip the project-cache lookup entirely and fall back to the VaneHub CLI profile default model

#### Scenario: `~/.claude.json` is malformed or absent
- **WHEN** `~/.claude.json` does not exist or contains invalid JSON
- **THEN** the system SHALL log a diagnostic warning and fall back to the VaneHub CLI profile default model

### Requirement: Discover model from Codex CLI native config
The system SHALL read the active model from Codex CLI's `config.toml` at `~/.codex/config.toml` when building session chat configuration defaults for the `codex-cli` agent.

#### Scenario: Model found in config.toml top level
- **WHEN** `~/.codex/config.toml` exists and contains top-level key `model = "gpt-5.4"`
- **THEN** the discovered model ID `gpt-5.4` SHALL be used as the session's initial model

#### Scenario: Model missing from config.toml
- **WHEN** `~/.codex/config.toml` exists but does not contain a top-level `model` key
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `codex-cli`

#### Scenario: Config file absent
- **WHEN** `~/.codex/config.toml` does not exist
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `codex-cli`

#### Scenario: Config file is malformed TOML
- **WHEN** `~/.codex/config.toml` exists but contains invalid TOML
- **THEN** the system SHALL log a diagnostic warning and fall back to the VaneHub CLI profile default model

### Requirement: Prefer Codex CLI's project-scoped model override
`~/.codex/config.toml` MAY contain a `[projects.'<path>']` table per trusted project. When such a table exists for the session's workspace path and contains its own `model` key, the system SHALL prefer it over the file's top-level `model`.

#### Scenario: Project section defines its own model
- **WHEN** `~/.codex/config.toml` has a `[projects.'<path>']` table matching the session's workspace path with `model = "deepseek-v4-pro"`, and the file's top-level `model` is a different value
- **THEN** the discovered model ID SHALL be `deepseek-v4-pro`

#### Scenario: Project section exists without a model override
- **WHEN** the matched `[projects.'<path>']` table exists (e.g. it only sets `trust_level`) but has no `model` key
- **THEN** the system SHALL fall back to the file's top-level `model`

#### Scenario: Workspace path does not match any project section
- **WHEN** no `[projects.'<path>']` table matches the session's workspace path (after the same normalization used for Claude Code's project cache)
- **THEN** the system SHALL use the file's top-level `model`

### Requirement: Discover model from Gemini CLI native config
The system SHALL read the active model from Gemini CLI's `.env` file at `~/.gemini/.env` when building session chat configuration defaults for the `gemini-cli` agent.

#### Scenario: Model found in .env file
- **WHEN** `~/.gemini/.env` exists and contains the line `GEMINI_MODEL=gemini-2.5-flash`
- **THEN** the discovered model ID `gemini-2-5-flash` (with dots normalized to hyphens) SHALL be used as the session's initial model

#### Scenario: GEMINI_MODEL not set in .env
- **WHEN** `~/.gemini/.env` exists but does not contain a `GEMINI_MODEL` key
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `gemini-cli`

#### Scenario: Config file absent
- **WHEN** `~/.gemini/.env` does not exist
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `gemini-cli`

### Requirement: Discover model from OpenCode native config
The system SHALL read the active model from OpenCode's `opencode.json` at `~/.config/opencode/opencode.json` when building session chat configuration defaults for the `opencode` agent.

#### Scenario: Single provider with one model
- **WHEN** `~/.config/opencode/opencode.json` exists with a single provider containing one model key under `provider.<id>.models`
- **THEN** that model key's name SHALL be used as the session's initial model

#### Scenario: Multiple providers present
- **WHEN** `~/.config/opencode/opencode.json` exists with multiple providers
- **THEN** the system SHALL select the first provider listed and use its first model key as the discovered model ID

#### Scenario: Config file absent
- **WHEN** `~/.config/opencode/opencode.json` does not exist
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `opencode`

#### Scenario: Config file is malformed JSON5
- **WHEN** `~/.config/opencode/opencode.json` exists but contains invalid JSON5
- **THEN** the system SHALL log a diagnostic warning and fall back to the VaneHub CLI profile default model

### Requirement: Prefer OpenCode's actual session model over its static catalog
`opencode.json` only declares which models a provider makes available; it does not record which one is selected. The system SHALL prefer the model recorded in OpenCode's own SQLite state (`~/.local/share/opencode/opencode.db`, table `session`, columns `directory`/`model`/`time_updated`) for the most recently updated session matching the workspace path, opened read-only, before falling back to the static catalog.

#### Scenario: A session exists for the workspace directory
- **WHEN** `opencode.db`'s `session` table has one or more rows whose `directory` matches the session's workspace path (after normalization) and whose `model` is non-null
- **THEN** the system SHALL use the `id` field from the most recently updated (`time_updated` descending) matching row's `model` JSON as the discovered model ID

#### Scenario: Multiple sessions exist for the same directory
- **WHEN** more than one `session` row matches the workspace directory with different `model` values
- **THEN** the system SHALL use the model from the row with the greatest `time_updated`

#### Scenario: No session matches the workspace directory
- **WHEN** no `session` row's `directory` matches the workspace path
- **THEN** the system SHALL fall back to `opencode.json`'s first provider's first model key

#### Scenario: Database is absent, locked, or unreadable
- **WHEN** `opencode.db` does not exist, cannot be opened read-only within a bounded timeout, or a matched row's `model` is not valid JSON
- **THEN** the system SHALL log a diagnostic warning and fall back to `opencode.json`'s first provider's first model key

### Requirement: Native model discovery does not block session creation
A failure to read or parse any CLI's native configuration file SHALL NOT prevent session creation or produce a user-visible error. Discovery failures SHALL be recorded as diagnostic warnings only.

#### Scenario: Discovery fails for any reason
- **WHEN** any native config read fails (missing file, permission denied, malformed content, or parse error)
- **THEN** session creation SHALL proceed normally with the VaneHub CLI profile default model
- **AND** the failure SHALL be logged as a diagnostic warning through the unified logging service

### Requirement: Discovery respects deterministic configuration precedence
The discovered native model SHALL serve as the initial value only. If the user has explicitly persisted a model selection via the VaneHub chat configuration, that persisted value SHALL take precedence over the discovered native model.

#### Scenario: User has explicitly selected a model in VaneHub
- **WHEN** a session has a persisted chat configuration with an explicit model ID (not the default)
- **THEN** the persisted model ID SHALL be used regardless of what the native config file contains

#### Scenario: User has never changed the model in VaneHub
- **WHEN** a session does not have an explicitly persisted model override
- **THEN** the discovered native model SHALL be used as the initial effective model

### Requirement: Web/mock runtime returns simulated discovered models
The Web/mock agent client SHALL return a deterministic simulated model for each agent to maintain parity with the desktop runtime's discovery behavior.

#### Scenario: Web mock session defaults
- **WHEN** a session is created via the Web/mock adapter
- **THEN** the adapter SHALL return a mock discovered model that matches the agent's catalog default
- **AND** the return format SHALL be identical to the desktop runtime's ChatConfig DTO

### Requirement: Discover model from Antigravity CLI native config
The system SHALL read the active model from Antigravity CLI's settings document at `~/.gemini/antigravity-cli/settings.json` when building session chat configuration defaults for the `antigravity-cli` agent.

#### Scenario: Model found in settings document
- **WHEN** `~/.gemini/antigravity-cli/settings.json` exists and records a model value
- **THEN** that model SHALL be used as the session's initial model

#### Scenario: Settings document has no model key
- **WHEN** `~/.gemini/antigravity-cli/settings.json` exists but records no model value
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `antigravity-cli`

#### Scenario: Settings document absent
- **WHEN** `~/.gemini/antigravity-cli/settings.json` does not exist
- **THEN** the system SHALL fall back to the VaneHub CLI profile default model for `antigravity-cli`

#### Scenario: Settings document is malformed
- **WHEN** `~/.gemini/antigravity-cli/settings.json` exists but contains invalid JSON
- **THEN** the system SHALL log a diagnostic warning and fall back to the VaneHub CLI profile default model for `antigravity-cli`
- **AND** it SHALL NOT modify the file

