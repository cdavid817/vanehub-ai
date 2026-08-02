## Why

VaneHub can detect, install, and launch Claude Code, OpenCode, and Codex CLI, but users still have to edit each CLI's user-level configuration files manually when changing providers, endpoints, credentials, or models. The Agents page needs a safe configuration-profile workflow, modeled after CC Switch's live-config projection approach, so one saved profile can be applied to the corresponding CLI's global user configuration without leaving VaneHub.

## What Changes

- Add saved configuration profiles for the `claude-code`, `opencode`, and `codex-cli` registered agents, including provider endpoint, credential reference, model settings, and agent-specific advanced configuration.
- Bundle a versioned common-provider preset catalog for Anthropic/OpenAI official configuration, OpenRouter, DeepSeek, Zhipu GLM, Kimi/Moonshot, SiliconFlow, Alibaba Bailian, and Volcengine Ark. Show only Agent-compatible presets and turn a selected preset into an editable user-owned profile without embedding credentials.
- Add a dedicated Agent configuration management page for profile create, import-current, edit, duplicate, delete, validate, and apply actions. Following CC Switch's provider-first information hierarchy, the page centers a compact Agent switcher and saved-profile list, while provider presets move into the create flow instead of remaining beside the list. The Agents page retains a clear entry and applied/global state remains separate from runtime Agent selection.
- Synchronize standard user-level CLI configurations using mode semantics inspired by CC Switch, without introducing a compatibility contract: on desktop startup Claude Code and Codex import one `default` profile only when that Agent has no saved profiles, while OpenCode idempotently upserts every supported live `provider` entry by provider id on every desktop startup.
- Implement desktop-native global configuration projection for Claude Code `settings.json`, Codex `config.toml` plus credential handling, and OpenCode `opencode.json`.
- Preserve unmanaged user configuration, automatically backfill externally edited managed values into the leaving Claude Code or Codex profile before a different profile is applied, serialize switches per Agent, use atomic file replacement, and compensate multi-file failures so a failed switch does not leave a partially applied configuration.
- Treat Claude Code and Codex as exclusive active-profile projections while preserving OpenCode's additive provider catalog semantics; applying an OpenCode profile ensures that provider definition is globally available instead of deleting unrelated providers.
- Store profile metadata in SQLite and secrets in the platform credential store. Secrets may be materialized only into the CLI-owned live files required by the selected CLI and must never appear in frontend reads or persisted logs.
- Provide deterministic Web/mock profile management and switch simulation without reading or writing local CLI files; Web mode reports local discovery as unavailable instead of fabricating candidates.
- Keep global configuration switching independent from active Session/workflow selection and report when a running CLI process must be restarted to observe the new configuration.
- Exclude Gemini CLI, remote preset marketplaces or automatic catalog downloads, local proxy takeover, and cross-device synchronization from this change.

## Capabilities

### New Capabilities

- `cli-agent-config-management`: Manage per-Agent CLI configuration profiles and safely project a selected profile into Claude Code, OpenCode, or Codex user-level global configuration.

### Modified Capabilities

- `agent-switching`: Distinguish user-level CLI configuration activation in Agent settings from runtime Agent and Session selection so applying a profile cannot silently change the active workflow.

## Impact

- Frontend: a lazy-loaded Agent configuration settings page with a compact Claude Code/OpenCode/Codex switcher, lightweight status and startup-synchronization strip, saved-provider profile cards, focused toolbar, and a large create/edit dialog containing compatible preset discovery plus the selected Agent-specific form. Applied profiles receive persistent visual emphasis; destructive actions retain accessible confirmation dialogs and narrow layouts remain supported. Existing service contracts remain implemented by both Tauri and Web adapters.
- Native runtime: new tooling/CLI application ports and Tauri commands for profile persistence, startup live-config synchronization, switch-away backfill, live-config inspection, validation, credential handling, locking, atomic projection, rollback, and unified logging. Synchronization is event-driven at startup or profile switch and does not add a resident file watcher.
- Storage: additive SQLite tables for profile metadata and applied-state fingerprints; credentials remain in the OS credential store and are omitted from DTOs.
- Filesystem: desktop-only writes to the resolved user-level Claude Code, OpenCode, and Codex configuration files while preserving fields outside VaneHub's owned projection.
- Architecture: React remains behind the frontend service boundary; native path resolution and file mutation remain in Rust. The Web runtime simulates the same contract without claiming to have changed local global configuration.
