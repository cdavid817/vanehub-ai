## Why

Google shut Gemini CLI down for individual Pro and Ultra users on 2026-06-18 and published Antigravity CLI (`agy`) as its successor — a Go binary that speaks the same Antigravity agent harness as the Antigravity 2.0 desktop IDE. VaneHub AI ships `gemini-cli` as one of its four built-in CLI agents, so the Google column of the product is now pinned to a sunset tool. Adding `antigravity-cli` as a fifth built-in CLI restores a supported Google path.

Antigravity CLI is not a drop-in reshaping of an existing integration. It is the first built-in CLI that has no npm package, no winget package, no API-key authentication, and no usable third-party endpoint redirection — every one of those breaks an assumption currently baked into the CLI-facing capabilities. This proposal covers the integration and the assumptions it forces open.

## What Changes

- **Register `antigravity-cli` as a built-in CLI agent** (executable `agy`, provider Google, interaction mode `cli`) in the native agent catalog, with a SQLite migration that back-fills the row for existing installs, plus brand icon and five-locale strings.
- **Add the managed CLI chat invocation contract**: `agy [managed parameters] --conversation <id> -p <prompt> --output-format stream-json`, prompt delivered as an argument (not stdin), resumed by conversation id.
- **Add NDJSON output parsing** for Antigravity's three event kinds — `init` (carries `conversation_id`), `step_update` (incremental steps and tool calls), `result` (terminal status, response text, usage). Only `result.status == SUCCESS` completes successfully; `ERROR`, `CANCELED`, `INTERRUPTED`, and `INVALID` each map to a lifecycle failure state that preserves the reported error.
- **Extend CLI installation management to script-only CLIs.** `ToolDefinition.package_name` becomes optional (Antigravity has no npm package), and a PowerShell installer URL joins the existing shell installer URL so Windows has an automated install path — today a CLI without an npm package and without a winget package would have none, because the existing script-install path runs through `bash -lc`.
- **Add a fourth CLI configuration profile kind, `antigravity`,** managing the settings Antigravity actually exposes (`toolPermission`, `enableTerminalSandbox`, `verbosity`, default model, plus pass-through for unmodelled keys) in `~/.gemini/antigravity-cli/settings.json`. **This kind carries no credential**: Antigravity authenticates through the OS keyring and Google Sign-In, and ignores `GEMINI_API_KEY` entirely, so credential capture, credential validation, and the `needs-credential` validation state do not apply to it.
- **Deliberately not doing provider-endpoint switching for this CLI.** The other three profile kinds exist to point a CLI at a custom `baseUrl` plus model plus token. Antigravity speaks Google CodeAssist's `cloudcode-pa.googleapis.com` surface, not the OpenAI Chat/Responses, Anthropic Messages, or Gemini API formats, and although `CLOUD_CODE_URL` does override the endpoint, the binary still requires a valid Google OAuth token before it issues any request. A relay profile for this CLI would be a control that cannot work.
- **Add the fifth typed launch-parameter profile**: `--model` (custom-text), `--effort` (`low`/`medium`/`high`), `--agent` (custom-text), and `--sandbox` (boolean). `-p`, `--output-format`, and `--conversation` stay managed and unselectable — a user selection would collide with the invocation contract — and `--dangerously-skip-permissions` stays out entirely, matching the existing catalog-wide prohibition on bypass flags that the other four CLIs already observe. A permissive posture is reachable only through the configuration profile's `toolPermission` setting.
- **Project policy templates into Antigravity's own approval controls**, joining `codex-cli`, `gemini-cli`, and `opencode`, so the agent appears as a governed principal rather than an ungoverned exception.
- **Read the active model** from `~/.gemini/antigravity-cli/settings.json` when building session chat configuration defaults.
- **Ingest reported token usage** from the `result` event's `usage` object, folding `thinking_tokens` into the output count consistent with how codex-cli, opencode, and gemini-cli already fold their reasoning tokens.
- **Extend every remaining enumeration of the built-in CLI set** — shared memory pool, chat experience, session cards, prompt-hook bindings, skill mount defaults, permission principal list — so no surface silently treats the built-in CLI roster as four.

### Non-Goals

- **Interactive embedded-terminal reported-usage ingestion.** The managed (non-interactive) pipeline reports usage inline on the `result` event and is covered here. The post-hoc terminal-mode path used for the other CLIs requires knowing where Antigravity writes its own session transcripts; that location is not documented and no authenticated sample was available. Deferred as its own change, matching how `gemini-cli` terminal tracking was deferred out of `add-terminal-usage-tracking`.
- **Endpoint/relay profiles for `antigravity-cli`** (see rationale above).
- **Collapsing the several hard-coded built-in CLI id arrays into one capability matrix.** Worth doing, but bundling a cross-cutting refactor into a feature addition puts both at risk. Separate change.

### Facts requiring live verification before implementation lands

The following are grounded in third-party reporting rather than official documentation, and are called out so implementation confirms them against a real install instead of inheriting a guess:

- The version-probe command (`agy --version`) and self-update command (`agy update`). Version probing degrades to the existing `VersionCheckStatus::Failed` path if wrong, so a mistake here is visible rather than silent.
- The exact field names inside `step_update` events. `init`, `result`, and the `usage` and status vocabularies are documented; the incremental event's payload shape is not.
- The exit code and stderr signature of an unauthenticated non-interactive run, needed to map "not signed in" to `needs-authentication` rather than a generic launch failure.

## Capabilities

### New Capabilities
(none — this change extends existing capabilities)

### Modified Capabilities
- `native-runtime-architecture`: registers `antigravity-cli` in the built-in agent catalog and adds its managed CLI chat invocation and NDJSON output-parsing contract; extends the CLI chat invocation enumerations that govern custom-instruction and memory injection.
- `agent-terminal-runtime`: adds `antigravity-cli` to interactive Agent Terminal startup, CLI parameter injection, and CLI executable resolution.
- `cli-agent-config-management`: extends the supported profile agent ids from three to four and introduces the credential-free `antigravity` profile kind targeting `~/.gemini/antigravity-cli/settings.json`.
- `cli-parameter-management`: adds the `antigravity-cli` typed launch-parameter profile.
- `settings-cli-management-ui`: defines lifecycle eligibility and install guidance for a CLI distributed only by installer script, with no npm package and a platform-specific Windows installer.
- `cli-agent-permission-launch-flags`: projects policy templates into Antigravity's native approval and sandbox controls.
- `permissions-approval`: the managed CLI principal list grows from four to five.
- `native-model-discovery`: adds active-model discovery from Antigravity's settings file.
- `usage-statistics`: adds managed-pipeline reported-token persistence for `antigravity-cli`.
- `agent-cross-session-memory`: adds `antigravity-cli` to the CLI-wrapped agents sharing the host-level memory pool.
- `chat-experience`: adds `antigravity-cli` to the CLI-backed session rendering enumeration.
- `main-layout-ui`: adds `antigravity-cli` to session card rendering.
- `prompt-hook-management`: adds `antigravity-cli` to the persistable stable CLI agent ids.
- `skill-management`: adds `antigravity-cli` skill mount defaults.

## Impact

**Runtimes:** both. Full behavior lives in the Tauri desktop runtime; the Web/mock adapter must gain the matching agent entry and mock data so `tauri-agent-client.ts` and `web-agent-client.ts` stay interface-identical.

**Rust (`src-tauri/`):**
- `contexts/agent_runtime/infrastructure/schema.rs` — built-in seed table entry; `platform/database/migrations.rs` — back-fill migration.
- `contexts/agent_runtime/infrastructure/providers/{invocation.rs,output.rs}` and `fixtures/invocations.json` — invocation contract and NDJSON parsing.
- `contexts/agent_runtime/infrastructure/terminal_usage_ingestion.rs` — managed-pipeline usage mapping.
- `contexts/tooling/cli/domain/mod.rs` — `ToolDefinition.package_name` becomes `Option`, new `powershell_install_url` field, new catalog entry (**touches all four existing entries' construction sites**); `infrastructure/{support.rs,package_adapter.rs,candidates.rs}` — install command derivation without an npm fallback, PowerShell install path, `agy` discovery paths.
- `contexts/tooling/cli_config/domain/mod.rs`, `infrastructure/live_config.rs` — supported id list, new payload variant, settings-file path and JSON fragment handling.
- `contexts/tooling/cli_parameters.rs` — parameter catalog and flag mapping.

**Frontend (`src/`):** `contracts/agent.ts` (`managedCliAgentIds`), `types/cli-agent-config.ts` (`cliConfigAgentIds`, new payload interface), `services/cli-parameter-catalog.ts`, `services/mock-agent-data.ts`, `settings/pages/agents/cli-config-*.tsx` (a payload-kind branch that renders no credential field), `lib/agent-visual-identity.ts`, `components/agent-brand-icon.tsx`, and all five locale files.

**Data:** one SQLite migration, no schema change beyond the seeded row. **Migration-number collision is a live hazard here** — every worktree on this machine shares one `%APPDATA%\ai.vanehub.app\vanehub.sqlite`, so a version number already claimed by another in-flight branch produces a startup crash rather than a merge conflict. The number must be chosen against the actual `schema_migrations` contents, not against this branch's file listing.

**External dependency:** installs and updates execute a Google-hosted installer script (`install.sh` / `install.ps1`), a distribution channel VaneHub already uses for Claude Code and OpenCode.
