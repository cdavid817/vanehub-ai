# CLI Agent global configuration

VaneHub manages user-level provider profiles for all five external CLI Agents from the Agent Management page. A saved profile is separate from the runtime Agent and Session selection. Saving or applying a profile never selects an Agent, changes the active Session, or restarts a running CLI process.

## Supported global files

| Agent | Resolved user-level file | Application semantics |
| --- | --- | --- |
| Claude Code | `~/.claude/settings.json` | Replaces only VaneHub-owned provider/model environment keys and preserves hooks, permissions, plugins, and unrelated environment values. |
| Codex CLI | `~/.codex/config.toml` | Selects one model provider and updates only the owned top-level/provider fields while preserving projects, MCP servers, comments, and unrelated providers. |
| Codex CLI | `~/.codex/auth.json` | Changed only by an explicit `replace-auth` profile after a separate confirmation; the prior bytes are restored if the multi-file switch fails. |
| OpenCode | `~/.config/opencode/opencode.json` | Adds or updates one provider and the global default model while preserving other providers, plugins, and settings. JSON5 input is accepted and normalized to JSON on write. |
| Gemini CLI | `~/.gemini/.env` | Updates only the owned endpoint, model, and authentication keys and preserves unrelated environment entries. |
| Antigravity CLI | `~/.gemini/antigravity-cli/settings.json` | Updates only the owned root-level keys — model, tool-approval mode, verbosity, terminal sandbox — and preserves everything else. Antigravity shares the `~/.gemini` home but keeps its own settings document. |

Claude Code and Codex keep many saved profiles but expose one globally applied profile. OpenCode keeps provider definitions additively and changes only its global default `provider/model` selection.

**Third-party endpoints are not universally available.** Claude Code, Codex CLI, and OpenCode accept any compatible endpoint from the shared provider directory. Gemini CLI accepts a custom base URL but ships only the Google official preset. For Antigravity CLI, VaneHub currently manages no endpoint or credential field: its managed profile edits only model and approval-related settings, and Google Sign-In credentials live in the operating-system keyring owned by the CLI itself. This describes VaneHub's current management scope, not an upstream limitation — whether the Antigravity CLI itself accepts an API key or a custom endpoint is defined by its official documentation, and such settings can be configured in the CLI's own environment.

## Startup synchronization inspired by CC Switch

The desktop runtime reads standard CLI configuration paths during startup; it does not continuously watch them.

- Claude Code and Codex use exclusive mode. When an Agent has no saved profiles, one `default` profile is created from its parseable live configuration and recorded as applied without rewriting the live file. Once any profile exists, later startups skip this import.
- OpenCode uses additive mode. Every startup parses all supported entries under `opencode.json.provider`, creates profiles for new provider ids, and updates matching profiles when their live values or credentials changed. A database profile is not deleted merely because its provider is absent from the current live file.
- Missing or malformed files affect only that Agent's best-effort synchronization pass. Warnings are redacted and do not block desktop startup or another Agent's synchronization.

The status strip reports the latest startup synchronization outcome. The separate “Import current configuration” action remains available as an explicit recovery or copy operation when a custom profile name is needed. No raw configuration content or credential value is returned to React.

## Bundled presets

CLI presets are derived from the same 25-entry provider directory the native OnePiece Agent uses, so the two stay in step rather than drifting apart. It covers Anthropic and OpenAI official configuration alongside OpenRouter, DeepSeek, Zhipu GLM, Kimi, Moonshot, SiliconFlow, Alibaba Bailian, Volcengine Ark, Groq, xAI, Mistral AI, Together AI, Fireworks AI, NVIDIA NIM, Cerebras, MiniMax, StepFun, Baichuan AI, PPIO, Qiniu AI, ModelScope, Xiaomi MiMo, and Z.AI. Each directory entry yields a preset per Agent whose endpoint protocol it matches — Anthropic-messages endpoints produce Claude Code presets, OpenAI-responses and chat-completions endpoints produce Codex and OpenCode presets. Gemini CLI and Antigravity CLI instead carry a single official preset each. The UI also offers a custom provider.

Presets contain only display metadata, endpoint/protocol defaults, recommended model ids, and normalized non-secret fields. Selecting a preset creates an editable user-owned profile; it does not write a CLI file. A later VaneHub catalog update never mutates an existing profile. Endpoint and model offerings can change between releases, so users can review and edit every preset value before saving.

## Credentials and privacy

Profile metadata and normalized non-secret payloads are stored in SQLite. Credentials are stored under an Agent/profile-scoped account in the operating-system credential service. Frontend responses expose only `credentialConfigured`; API keys, authorization values, and reversible credential references are not returned or stored in Web mode.

When a CLI requires a plaintext credential in its own live configuration, the desktop runtime materializes it only during an explicit global apply. Unified logs contain safe operation, Agent, and profile identifiers but never configuration bodies or credentials.

## External edits, writes, and recovery

After a successful apply, VaneHub fingerprints the managed fragment. When switching from one Claude Code or Codex profile to another, VaneHub re-reads the live managed fields and automatically backfills them into the leaving profile before projecting the target. Credentials are replaced through the operating-system credential service, never through SQLite. If parsing, profile persistence, or credential compensation fails, the target live file is not changed.

Fingerprints remain visible as drift status and protect the narrow race between planning and atomic replacement. A live change that races an in-progress switch aborts the write. OpenCode external edits are incorporated on the next desktop startup or explicit manual import, not in real time.

Desktop applications are serialized per Agent. VaneHub validates and builds all output in memory, snapshots the exact previous bytes or absence, writes a sibling temporary file, and atomically replaces the target. Codex multi-file failures restore every file already changed. If applied-state persistence fails after projection, the live files are restored before the error is returned.

Successful application reports that running CLI processes may need to be restarted. VaneHub does not claim hot reload and does not terminate those processes automatically.

## Web behavior

The Web adapter provides deterministic profile and preset management for UI development. It discards submitted credential values, records only credential presence, returns `simulated: true`, reports no affected filesystem paths, and leaves workflow and Session state unchanged. Startup synchronization is explicitly `unavailable` in Web mode and never fabricates local profiles, paths, or candidates.

## Native parser dependency review

- `toml_edit 0.25.12`: already present in the locked dependency graph and added directly for syntax-aware Codex edits that retain unrelated tables and comments. It is used only in the native CLI configuration adapter.
- `json5 0.4.1`: added for parsing OpenCode's supported JSON5 input before emitting validated JSON. It is a parser-only dependency and receives no network, process, or filesystem capability.
- `windows-sys 0.61`: already present in the locked dependency graph and added directly with only `Win32_Storage_FileSystem` to use `MoveFileExW` with replace/write-through flags on Windows.

All versions and checksums are pinned by `src-tauri/Cargo.lock`. The catalog is compiled locally and has no remote update or executable-content path.
