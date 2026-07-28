## Why

VaneHub currently relies on an entirely hardcoded per-CLI model catalog. Each of the four managed CLIs—Claude Code, Codex CLI, Gemini CLI, and OpenCode—has a fixed list of supported models defined in Rust (`chat_configuration.rs`) and mirrored in the frontend (`models.ts`). When a user configures their CLI's native config file with a model outside VaneHub's catalog—for example, setting `"model": "deepseek-chat"` in `~/.claude/settings.json`—the information panel silently falls back to the default model label (e.g., "Opus 4.8"), displaying an incorrect model. All four CLIs support custom model values natively, but VaneHub cannot represent any of them.

The previous fix (adding `PROVIDER_MODELS` label lookup in the session info panel) only addressed the display layer for known models. Custom and third-party models configured in CLI native config files remain invisible and unrepresentable, and the VaneHub UI cannot be used to configure them.

## What Changes

- **Read live model from each CLI's native config**: The native runtime SHALL read the actual model value from each managed CLI's configuration file at session startup, using it as the source of truth for the session info panel and model selector. Per-CLI config paths:
  - Claude Code: `~/.claude/settings.json` → `env.ANTHROPIC_MODEL` (primary model env key)
  - Codex CLI: `~/.codex/config.toml` → top-level `model`
  - Gemini CLI: `~/.gemini/.env` → `GEMINI_MODEL`
  - OpenCode: `~/.config/opencode/opencode.json` → `provider.<active-id>.models` keys (json5 format)
- **Accept discovered models through the full pipeline**: When a discovered or user-entered model ID does not match any hardcoded catalog entry, the system SHALL accept it as a valid model rather than rejecting it or silently falling back to the default. This requires changes at every layer: CLI parameter validation, `model_id_from_cli`, `ChatAgent::supports`, and the frontend display pipeline.
- **Display friendly labels for known models, raw IDs for unknown ones**: The info panel and model selector SHALL look up friendly labels from the catalog when available, and fall back to the raw model ID (with dots/hyphens normalized for display) for models outside the catalog.
- **Add free-text model input to CLI parameter profiles**: The CLI parameter management page SHALL allow free-text model entry alongside the existing enum options for all four CLIs, enabling users to configure any model their CLI accepts.
- **Maintain backward compatibility**: Existing sessions with catalog models SHALL continue to display and function identically. The hardcoded catalog remains the source of friendly labels, reasoning depth limits, and long-context capability flags. Unknown models get sensible defaults (no reasoning, no long-context) rather than being rejected.

## Capabilities

### New Capabilities

- `native-model-discovery`: Read the currently active model from each CLI's native configuration file (Claude Code: `~/.claude/settings.json` → `env.ANTHROPIC_MODEL`; Codex CLI: `~/.codex/config.toml` → top-level `model`; Gemini CLI: `~/.gemini/.env` → `GEMINI_MODEL`; OpenCode: `~/.config/opencode/opencode.json` → `provider.<id>.models` keys) at session startup. Surface the discovered model ID through the existing chat configuration boundary. When config files are absent, unreadable, or the model field is missing, fall back to VaneHub's CLI profile default.
- `custom-model-display`: When a model ID is not found in the hardcoded `PROVIDER_MODELS` catalog, normalize it for display (capitalize words, replace hyphens/dots with spaces) and surface it as-is in the info panel and model selector, rather than silently substituting the CLI's default model. This applies to all four CLIs uniformly.

### Modified Capabilities

- `session-chat-configuration`: The model resolution pipeline (`model_id_from_cli`, `normalize_chat_preferences`, `ChatAgent::supports`) SHALL accept any non-empty model ID string instead of rejecting values not in the hardcoded catalog. The catalog remains the source of friendly labels, reasoning caps, and capability flags. Unknown models receive conservative defaults (reasoning depth clamped to "low", long-context disabled).
- `cli-parameter-management`: The `model` parameter control for all four managed CLIs SHALL change from `Enum` to a composite control that presents known catalog values in a dropdown plus a free-text field for arbitrary model IDs. Validation SHALL reject control characters and empty strings but SHALL accept any otherwise-valid model identifier string.

## Impact

- **Rust**: `chat_configuration.rs` (domain: `ChatAgent::supports` becomes permissive, `model_id_from_cli` accepts passthrough), `chat_profile.rs` (profile adapter: falls through to raw string when not mapped), `cli_parameters.rs` (model parameter: Enum → custom-text composite, validation relaxed), new `tooling/cli/infrastructure/native_config_reader.rs` (reads per-CLI native config files)
- **Frontend**: `models.ts` (add `resolveModelLabel()` helper, accept unknown IDs), `session-info-panel.tsx` (use resolved label), `ModelSelect.tsx` (show custom model IDs when not in catalog), CLI parameter settings page (free-text model input control), `web-agent-client.ts` (return mock discovered model)
- **Both runtimes**: Desktop runtime reads actual config files per CLI; Web/mock adapter returns simulated discovered models
- **No breaking changes**: Catalog models retain existing behavior and labels; custom models are additive; all existing Tauri commands preserve their contract
