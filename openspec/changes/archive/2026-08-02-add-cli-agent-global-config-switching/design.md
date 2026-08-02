## Context

VaneHub currently has three adjacent concepts that must remain distinct:

1. the registered Agent (`claude-code`, `opencode`, or `codex-cli`),
2. the CLI launch-parameter profile used when VaneHub starts a process, and
3. the provider/model configuration read by the CLI from the user's global config directory.

The requested feature concerns the third concept. Today the native config reader can inspect model metadata from `~/.claude/settings.json`, `~/.codex/config.toml`, and `~/.config/opencode/opencode.json`, but no service owns profile persistence or safe writes to those files. The Agents page also exposes runtime workflow selection, so the new UI must avoid presenting global configuration activation as if it switched the active Session.

CC Switch provides the reference behavior: store provider profiles separately, backfill managed live changes before leaving a profile, and project a selected profile into each CLI's live files. Its important agent-specific distinction is retained: Claude Code and Codex use an exclusive live configuration, while OpenCode keeps a provider catalog in an additive JSON document.

The feature crosses React, the frontend service adapters, Tauri commands, SQLite, the OS credential store, and user-owned configuration files. It therefore needs explicit ownership, compensation, and redaction rules.

## Goals / Non-Goals

**Goals:**

- Manage multiple named global configuration profiles for Claude Code, OpenCode, and Codex CLI from a dedicated Agent configuration settings page that remains reachable from the Agents page.
- Let users start from a bundled common-provider preset that is compatible with the selected Agent, then review and edit it as a normal user-owned profile.
- Import the current live configuration into a profile without returning secrets to the frontend.
- On desktop startup, bootstrap Claude Code and Codex from live configuration only when that Agent has no saved profiles, and idempotently upsert compatible OpenCode live providers on every startup.
- Backfill externally edited managed values into the current exclusive profile immediately before switching away from it.
- Validate, apply, and inspect profiles through one frontend contract implemented by Tauri and Web/mock adapters.
- Preserve unrelated user configuration and detect when an externally edited live file no longer matches the last VaneHub projection.
- Make each desktop write atomic and make a multi-file switch recover the previous bytes when a later step fails.
- Keep credentials out of SQLite, DTOs, diagnostic text, and operation logs.
- Keep configuration activation independent from Agent/Session runtime selection.

**Non-Goals:**

- Gemini CLI configuration management.
- A remote provider marketplace, runtime catalog download, or remotely executable preset content.
- Protocol conversion, a local reverse proxy, health checking, or automatic failover.
- Synchronizing profiles between machines.
- Editing project-local configuration or rewriting already-running CLI processes.
- Selecting arbitrary configuration files or continuously watching CLI configuration files while the application is running.
- Replacing CLI installation/version management or CLI launch-parameter profiles.

## Decisions

### 1. Place global CLI configuration in the tooling context

The Rust implementation will live under a new `contexts/tooling/cli_config` module with domain, application, and infrastructure layers. It will reuse stable registered Agent ids but will not live in `agent_runtime`, because it mutates CLI-owned user configuration rather than executing a Session.

Tauri commands will be grouped under `commands/tooling/cli_config`. React will call methods added to the frontend service boundary; only `tauri-agent-client.ts` may invoke the native commands. `web-agent-client.ts` will implement the same methods against deterministic in-memory profiles.

Alternative considered: extend `contexts/agent_runtime`. Rejected because applying a global profile must not mutate workflow state, start a process, or inherit generation lifecycle concerns.

### 2. Store profiles separately from applied state

SQLite will add two structures:

- `cli_config_profiles`: stable profile id, stable Agent id, display name, non-secret normalized payload, managed-key manifest, timestamps, and sort position.
- `cli_config_applied_state`: one row per supported Agent containing the last applied profile id, projection fingerprint, live fingerprint, drift state, and application time.

The profile id is stable across rename and uses kebab-case plus collision suffixing. A profile payload is tagged by Agent kind so invalid cross-Agent data cannot be applied. Applied state is separate because OpenCode's live file may contain several providers and because a profile can be edited without immediately changing the global file.

Secrets are stored with the platform credential service under a scoped key derived from Agent id and profile id. Frontend models expose only `credentialConfigured: boolean`. Deleting a profile removes its credential only after reference and applied-state checks succeed; failures use compensation or return an actionable incomplete-cleanup error.

### 3. Use agent-specific projection adapters behind one application port

The application layer will select a `CliGlobalConfigAdapter` by stable Agent id. Unsupported ids, including `gemini-cli`, fail before any file read or write.

| Agent id | Live files | Projection semantics |
| --- | --- | --- |
| `claude-code` | `~/.claude/settings.json` | Exclusive VaneHub profile. Merge the profile-owned `env` keys and supported model keys; remove keys owned by the previously applied VaneHub profile; preserve all other top-level settings, hooks, permissions, plugins, and unrelated environment keys. |
| `codex-cli` | `~/.codex/config.toml`, and `auth.json` only when required by the selected credential strategy | Exclusive VaneHub profile. Use syntax-aware TOML editing for owned top-level keys and the selected `model_providers.<id>` table; preserve unrelated tables including MCP/project settings. Official ChatGPT login material is preserved by default and may only be replaced after explicit confirmation. |
| `opencode` | `~/.config/opencode/opencode.json` | Additive provider catalog. Upsert the selected `provider.<id>` fragment, preserve unrelated providers/plugins/settings, and set the global default `model` to the profile's declared provider/model. Previously imported non-VaneHub providers are never removed implicitly. |

Paths are resolved in Rust from the effective user home and existing project-native rules. Path overrides and WSL-specific management are deferred; the status DTO always shows the resolved target path before application.

Alternative considered: store and replace whole files. Rejected because it would erase user-managed hooks, MCP servers, permissions, comments/settings, and unrelated OpenCode providers.

### 4. Represent profiles as normalized, agent-specific payloads

The service contract will use a discriminated payload rather than raw arbitrary file contents:

- Claude Code: base URL, authentication mode, credential presence, primary/role model ids, and a bounded advanced environment map.
- Codex: model provider id, base URL, model id, wire API, reasoning settings, official-auth preservation policy, and a bounded advanced TOML fragment limited to the owned provider table/top-level allowlist.
- OpenCode: provider id, npm package, base URL, credential presence, headers, one or more model definitions, and a required default model.

The backend validates ids, URLs, field sizes, duplicate keys, TOML/JSON syntax, and ownership boundaries. Control characters and path-like provider ids are rejected. Raw advanced fragments cannot declare MCP, project, plugin, hook, or other out-of-scope sections.

Alternative considered: a raw JSON/TOML editor only. Rejected because it makes safe merging, secret extraction, Web parity, and field-level validation unreliable. An advanced editor may render the bounded normalized fragment but does not bypass validation.

### 5. Bundle a versioned, secret-free provider preset catalog

VaneHub will ship a typed local catalog with initial presets for:

- official Anthropic configuration where Claude Code supports it, and official OpenAI/Codex configuration where Codex or OpenCode supports it;
- OpenRouter;
- DeepSeek;
- Zhipu GLM, including separate China and international endpoints where their settings differ;
- Kimi/Moonshot, including a coding-plan variant where supported;
- SiliconFlow;
- Alibaba Bailian/DashScope; and
- Volcengine Ark.

Each preset declares a stable preset id, catalog version, display metadata, compatible Agent ids, protocol/auth strategy, default endpoint, conservative recommended models, and an Agent-specific normalized payload template. The Agent configuration page filters and searches the catalog by stable Agent id and provider category; it never adapts an incompatible preset by guessing protocol fields. “Custom provider” remains available for every supported Agent.

Selecting a preset creates an ordinary editable profile. The profile records the source preset id and version for diagnostics but owns a copy of the values, so a later application upgrade never silently changes an existing user profile. A preset may be revised or deprecated in a future bundled catalog; the UI can recommend reviewing a newer version, but replacement requires explicit user action.

Catalog entries contain no credentials, credential references, executable scripts, remote markup, or unrestricted raw configuration. Model and endpoint defaults are validated exactly like manual input, and users can edit them before saving because provider offerings change independently of the VaneHub release. Tauri and Web adapters consume the same frontend-visible catalog data and profile-creation contract.

Alternative considered: copy CC Switch's complete preset set and update it remotely. Rejected for the first release because a smaller compatibility-tested catalog is easier to audit and a remote catalog would add trust, signing, rollback, and supply-chain requirements.

### 6. Startup synchronization and switch-away backfill follow CC Switch mode semantics

The database remains the profile-management source of truth, while standard CLI files are the live configuration consumed by each CLI. Native synchronization extracts only adapter-owned fields, moves detected credentials directly into the OS credential store, and persists only normalized non-secret payloads. Unknown live fields remain outside profile storage.

Claude Code and Codex use exclusive mode. During desktop startup, each Agent is considered independently. If it has no saved profile and its live configuration is parseable, the runtime imports one stable `default` profile and records it as the applied profile without rewriting the live file. Once any profile exists for that Agent, later startups skip this import. Missing files and unsupported empty configurations are no-ops; malformed files produce redacted warning logs without preventing application startup.

OpenCode uses additive mode. On every desktop startup, the runtime parses all supported entries under `opencode.json.provider` and upserts them by their provider id. A new id creates a profile; an existing profile with the same payload provider id updates its normalized payload, display name, and credential reference when live values changed. Providers absent from the current live file are not deleted from SQLite. One malformed provider entry is skipped with a warning when the remaining document is usable; a malformed document aborts only the OpenCode synchronization pass.

Before applying a different Claude Code or Codex profile, the coordinator re-reads the current live managed fragment and automatically saves it into the leaving applied profile, including credential replacement through the credential store. Adapter-owned projections such as MCP data or unrelated shared settings are stripped before storage. If the live file is missing or malformed, or backfill persistence/credential compensation fails, the switch aborts before writing the target profile. Applying the already-current profile does not backfill.

Managed-fragment fingerprints remain useful for status and compare-before-write race protection, but ordinary external drift no longer requires a user resolution dialog when switching: it is preserved by backfill. If the file changes after the switch plan is built and before atomic replacement, application still aborts rather than overwriting the racing edit.

The existing manual `import current` command remains available as a recovery/explicit-copy action. There is no resident file watcher: exclusive external edits are recognized on the next switch, and OpenCode external edits are recognized on the next desktop startup.

### 7. Apply profiles as observable, serialized operations

Applying a profile follows this state machine:

1. acquire a per-Agent native switch lock;
2. load and validate the profile and credential availability;
3. for an exclusive Agent switching away from an applied profile, backfill the current managed live values and credential into that leaving profile;
4. read the exact old bytes/existence state of every target file;
5. build and validate all new documents in memory;
6. atomically replace each target file using a sibling temporary file and rename;
7. if any later file fails, restore prior bytes/existence for earlier files;
8. persist applied state only after all required file writes succeed;
9. record redacted unified operation logs and return restart guidance.

The operation result contains an operation id, terminal status, affected paths, applied profile id, backfill outcome, warnings, and `restartRequired`. It never contains config bodies or credentials. Concurrent requests for the same Agent serialize; different Agents may apply independently.

Codex is the main multi-file case. A third-party provider should preserve official `auth.json` by using provider-scoped authentication where supported. If a selected profile must own `auth.json`, the UI requires a distinct confirmation and the adapter restores the previous file if `config.toml` fails.

### 8. A dedicated Agent configuration page separates global configuration from runtime state

Settings will expose a lazy-loaded “Agent configuration” page adjacent to the existing Agents page. The Agents page remains focused on availability, interaction modes, and Session/workflow actions, and provides a clear “Manage configurations” entry. When entered from a supported Agent card, the destination may preselect that stable Agent id without changing runtime selection.

The page adopts the useful information hierarchy from CC Switch without copying its branding or expanding into its proxy, usage, promotion, or universal-provider features. A compact Claude Code, OpenCode, and Codex segmented switcher sits at the top of the workspace. A focused toolbar provides profile search, add, optional manual import-current, and refresh actions. Agent switching changes only the configuration workspace and keeps stable Agent ids as the state key.

The normal page state is profile-first:

- a lightweight status strip shows applied/detached/drifted state, Web simulation, last-apply context, and resolved paths without occupying a large dashboard region;
- a compact startup-synchronization notice reports imported, updated, skipped, or parse-warning outcomes without requiring a candidate-selection flow;
- a single-column saved-profile list is the primary content rather than a permanent side-by-side preset catalog;
- each profile card shows a deterministic provider avatar, name, endpoint, primary/default model, credential and validation state, source preset metadata where useful, and apply/edit/duplicate/delete actions;
- the applied profile receives persistent border/tone emphasis plus an explicit “Currently applied” badge, while hover alone is never the only state signal; and
- empty, filtered-empty, loading, operation-progress, rollback, and restart-guidance states remain inside the same workspace.

The add action opens a large accessible create dialog. Its upper section contains the compatible preset catalog as a searchable, categorized responsive chip/grid selector with a custom-provider entry. Selecting a preset immediately populates the Agent-specific form below it but never saves or applies automatically. The dialog keeps its cancel/save actions visible in a sticky footer. Editing reuses the form-oriented dialog but does not make the user reselect a preset or imply that source-preset changes will overwrite owned values. Credentials are never repopulated.

Apply and delete use application-owned confirmation dialogs instead of browser prompts; manual import uses an application dialog when user input is required. Dialogs restore focus, support keyboard dismissal when safe, and prevent double submission while an operation is pending.

Labels use “Apply globally”/“Applied globally” rather than the runtime “Configure” action. Applying a profile never calls `selectAgent`, changes `workflow_state`, switches a Session, or launches a process. Running processes are not killed; successful results state that a new process or restart may be required.

On narrow layouts the segmented switcher remains usable, toolbar actions wrap or collapse without hiding add/import, profile metadata stacks above its actions, and dialogs keep the preset selector, form, and sticky footer within the viewport without horizontal overflow. The design does not show a drag handle because this change has no reorder service contract. Page, switcher, toolbar, status, profile-card, preset-selector, form, and dialog components remain focused and below the project file-size limit.

### 9. Add only parser support required for safe live edits

The native layer will use a syntax-aware TOML editor for Codex and a JSON5-capable parser for OpenCode. Any new Rust dependency must be pinned through `Cargo.lock`, reviewed under the software-supply-chain rules, and used only in the native adapter. JSON output may normalize formatting, but semantic values outside VaneHub's managed fragment must be preserved.

### 10. Logging and privacy are metadata-only

Every create/update/delete/import/apply/drift action emits unified logs at the appropriate level with Agent id, profile id, operation id, result, and redacted target path. API keys, authorization headers, config bodies, TOML/JSON fragments, and credential-store errors containing secret material are never logged. Page-visible operation output remains available through the operation service.

## Risks / Trade-offs

- [External CLI changes its config schema] -> Version each normalized payload, validate before writing, keep adapters isolated, and refuse unknown incompatible shapes without changing live files.
- [Cross-file Codex write partially fails] -> Prebuild all output, snapshot exact old bytes in memory, atomically replace, and compensate before returning failure.
- [Concurrent VaneHub or external edits race with apply] -> Serialize VaneHub switches per Agent, backfill before planning the target write, and compare the pre-write fingerprint immediately before replacement; return a drift conflict only for a change racing the in-progress operation.
- [A CLI requires plaintext credentials in its own config] -> Keep the source secret in the OS credential store, materialize only on explicit apply, restrict file permissions where supported, and disclose the target paths before confirmation.
- [JSON/TOML normalization changes formatting] -> Preserve semantic unmanaged values; syntax-aware Codex editing minimizes churn, while OpenCode formatting normalization is accepted and shown in the apply preview.
- [User mistakes global profile activation for Session switching] -> Separate UI sections, actions, query keys, and service methods; add an invariant test that workflow/session state is unchanged.
- [Running CLI does not reload configuration] -> Never claim hot reload; return per-Agent restart guidance after a successful apply.
- [Bundled endpoint or model becomes stale] -> Keep presets versioned and editable, validate at profile creation/application, allow manual values, and never mutate an existing profile during a catalog upgrade.
- [Startup synchronization overwrites an intentional database-only OpenCode edit] -> Match by provider id only, document that a matching OpenCode profile follows the live file at startup, preserve database entries missing from live, and require a distinct provider id for a database-only variant.

## Migration Plan

1. Add the SQLite tables and indexes without changing existing Agent, workflow, or CLI parameter rows.
2. Run best-effort startup synchronization after database and unified logging initialization: exclusive Agents bootstrap only when empty, while OpenCode performs idempotent upsert.
3. Enable create/edit/apply actions for the three supported stable Agent ids and backfill the leaving exclusive profile during switches.
4. During every import or backfill, extract secrets directly into the credential store and persist only `credentialConfigured` state plus non-secret fields.
5. If the feature is rolled back, stop exposing the new commands and leave user live files untouched; the additive tables can remain dormant. No automatic restoration is performed because the last applied live configuration is a valid user configuration.

## Open Questions

- A future change may add configurable/WSL profile roots; this proposal uses the standard resolved user-level paths.
- Remote preset distribution and one profile spanning several Agents are intentionally deferred until the per-Agent projection contract is stable.
