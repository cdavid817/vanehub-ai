## Why

Skills currently cannot declare typed settings or receive project-specific values without hard-coded UI and prompt changes. A schema-driven configuration boundary is needed before configurable built-in, user-created, delegated, and self-evolved Skills can behave predictably without exposing credentials.

## What Changes

- Extend Skill frontmatter with a bounded `config_schema` using a supported JSON Schema subset and VaneHub annotations for labels, help text, ordering, advanced fields, and secret fields.
- Persist non-secret values through the Rust/SQLite layer at User and Project scope, store secret values only in the operating-system credential store, and resolve effective values as Project over User over schema defaults.
- Validate configuration on schema discovery, save, preview, effective resolution, and Skill invocation; preserve the last valid stored revision when a save fails.
- Bind effective configuration to the exact effective Skill and schema revision, surface drift and migration-required states, and never silently coerce incompatible values.
- Attach immutable configuration snapshots to Role loading, Utility delegation, and Skill tool execution while preventing secret values from entering prompts, frontend responses, logs, transcripts, evidence dossiers, or evolution candidates.
- Add a schema-generated configuration panel to Skill details with scope selection, inheritance visibility, validation, secret replacement/clearing, reset, stale-write protection, and honest Web-runtime behavior.
- Keep external CLI behavior explicit: configuration management is available centrally, but values are not projected into third-party CLI Skill files or processes without a separately specified bridge.

## Capabilities

### New Capabilities

- `skill-configuration-management`: Defines Skill configuration schemas, scoped persistence, precedence, validation, secret isolation, revision binding, runtime snapshots, and lifecycle behavior.

### Modified Capabilities

- `skill-management`: Adds configuration metadata and status to effective Skill revisions and integrates configuration lifecycle with Skill replacement, restore, archive, and deletion.
- `agent-skill-injection`: Supplies bounded non-secret effective configuration to eligible native API Skill contexts without leaking secrets or destabilizing unrelated prompt sections.
- `settings-skill-management-ui`: Adds schema-driven Skill configuration editing, inheritance, validation, secret management, drift handling, and adapter-parity behavior.

## Impact

- Desktop/native: adds configuration schema parsing, validation, scoped SQLite repositories, credential-store integration, effective snapshot resolution, Tauri commands, and unified-log events.
- Frontend: extends `AgentService` plus Tauri and Web adapters with generated configuration forms and redacted configuration state; React components continue to avoid direct native invocation.
- Runtime: native API Role/Utility and Skill tool contexts consume immutable snapshots. External CLI sessions receive no undeclared configuration projection.
- Storage and security: additive SQLite migration for non-secret values and revision witnesses; credential values remain outside SQLite, DTOs, Web storage, prompts, transcripts, and logs.
