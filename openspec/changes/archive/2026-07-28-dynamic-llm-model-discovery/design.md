## Context

VaneHub maintains a hardcoded model catalog for each of its four managed CLIs. The catalog serves three roles: friendly display labels, capability flags (reasoning depth, long-context), and validation (rejecting unknown model IDs). This worked when each CLI had 3-4 known models, but breaks as users adopt third-party API providers through relay/aggregator services or custom endpoints.

cc-switch solves this problem comprehensively—reading live config files, writing provider settings, supporting 50+ presets—but VaneHub's scope is narrower. This design focuses on the minimum changes needed to: (1) discover models from native config files, (2) accept and display custom model IDs, and (3) allow free-text model input in settings.

The architecture follows the existing DDD pattern: domain changes in `sessions` and `tooling` contexts, infrastructure adapters for file I/O, application ports for the discovery contract, and frontend changes behind the existing service boundary.

## Goals / Non-Goals

**Goals:**
- Read the active model from each CLI's native configuration file when composing session defaults
- Accept model IDs not in the hardcoded catalog through the full validation pipeline
- Display custom model IDs properly in the session info panel and model selector
- Allow free-text model input in the CLI parameter settings page

**Non-Goals:**
- Provider switching, API key management, or endpoint configuration (cc-switch's primary role)
- Writing models back to native config files (VaneHub is read-only for native configs)
- Auto-discovering available models from provider APIs
- Model catalog sync across machines
- MCP/proxy/failover management

## Decisions

### 1. Native config discovery as an application port in `tooling`

**Decision:** Create a `NativeConfigPort` trait in `tooling/cli/application/ports/` with a single method `discover_model(agent_id: &str, workspace_path: Option<&str>) -> Result<Option<String>>`. Implement it in `tooling/cli/infrastructure/native_config_reader.rs`.

**Rationale:** Follows the existing DDD dependency direction (commands → application → domain, infrastructure → application ports). The sessions context already depends on `tooling` via `CliParametersApi`; adding a native config read follows the same cross-context pattern via the `tooling` API facade. `workspace_path` is threaded through from the session's workspace (worktree path, falling back to project path) so per-project discovery sources (see Decision 9) can scope their lookup; CLIs without a per-project source simply ignore it.

**Alternatives considered:**
- Placing discovery in `sessions` context directly → violates ownership (file reading is tooling infrastructure)
- Reading config files in `chat_profile.rs` without a port → untestable without real filesystem

**Per-CLI read strategy:**

| CLI | Path | Format | Model Key | Parser |
|-----|------|--------|-----------|--------|
| claude-code | `~/.claude/settings.json` | JSON | `env.ANTHROPIC_MODEL` | `serde_json` |
| claude-code (fallback) | `~/.claude.json` | JSON | `projects[workspace_path].lastModelUsage` (single-key only) | `serde_json`, see Decision 9 |
| codex-cli | `~/.codex/config.toml` | TOML | top-level `model` | `toml` (already a dependency) |
| gemini-cli | `~/.gemini/.env` | KEY=VALUE | `GEMINI_MODEL` | manual line parsing |
| opencode | `~/.config/opencode/opencode.json` | JSON5 | `provider.<first-id>.models` keys (first model) | `serde_json` (tolerant) or manual key extraction |

Home directory resolved via `dirs::home_dir()` (Windows: `%USERPROFILE%`, macOS/Linux: `$HOME`).

### 2. Model passthrough in `model_id_from_cli`

**Decision:** Extend the function to return `Some(value)` for any non-empty, non-"default" string that doesn't match a known alias.

```rust
pub(crate) fn model_id_from_cli(agent_id: &str, model: &str) -> Option<&'static str> {
    match (agent_id, model) {
        // known aliases unchanged
        ("claude-code", "opus") => Some("claude-opus-4-8"),
        // ...
        // pass through unknown non-empty, non-"default" values
        (_, "") | (_, "default") => None,
        _ => Some(model), // lifetime issue — see decision #3
    }
}
```

**Issue:** The current signature returns `&'static str` because all mappings are to static string literals. Passthrough values are borrowed from the input, not static. **Rejected:** changing return type to `Option<String>` which clones. **Accepted:** changing return type to `Option<String>` and cloning the passthrough. The allocations are trivial (one per session startup) and the function currently clones downstream already anyway (`chat_profile.rs:34` calls `.map(str::to_string)`).

### 3. Permissive `ChatAgent::supports()`

**Decision:** Change `supports()` from a whitelist match to accept any non-empty model string.

```rust
fn supports(self, model_id: &str) -> bool {
    !model_id.trim().is_empty()
}
```

**Rationale:** The current function serves as a validation gate that rejects unknown models. Since the catalog is no longer the universe of valid models, this check becomes a simple non-empty guard. The provider check (`provider_id == agent.provider()`) remains strict and is the real safety boundary.

### 4. Conservative capability defaults for unknown models

**Decision:** `max_reasoning_for_model()` returns `None` for unknown models, causing `clamp_reasoning_for_model()` to return `None` as well (no reasoning). Frontend: `supportsReasoning: false`, `supportsLongContext: false`.

**Rationale:** We can't know the capabilities of arbitrary third-party models. Defaulting to conservative settings avoids sending unsupported parameters that might cause errors. The user can still manually enable reasoning or long-context if their model supports it.

### 5. Custom-text CLI parameter control kind

**Decision:** Add `CustomText` variant to `CliParameterControl` enum. It carries known enum values (`options`) for the dropdown but accepts arbitrary non-empty, non-control-char strings.

```
pub(crate) enum CliParameterControl {
    Enum,
    Boolean,
    MultiEnum,
    CustomText,  // NEW
}
```

**Frontend:** A composite component renders a `<select>` of known values plus a "Custom…" option. Selecting "Custom…" reveals a `<input type="text">`. The `model` parameter for claude-code, codex-cli, and gemini-cli uses `CustomText`. OpenCode has no `model` CLI parameter (it uses agent selection instead), so it's unaffected.

**Validation:** `validate_value` for `CustomText`:
- Rejects empty/whitespace-only → normalized to default
- Rejects values with control characters (`char::is_control`)
- Accepts everything else

**Argument preview:** `preview_args` for `CustomText`: renders `--model <value>` when value is not "default", same as `Enum`.

### 6. Frontend `resolveModelLabel` utility

**Decision:** Single exported function in `src/components/chat/models.ts`:

```typescript
export function resolveModelLabel(providerId: string, modelId?: string | null): string {
  if (!modelId) {
    // return default label for provider
  }
  const catalogModel = PROVIDER_MODELS[providerId]?.find(m => m.id === modelId);
  if (catalogModel) return catalogModel.label;
  // Normalize: split on dots/hyphens, capitalize each word, join with spaces
  return modelId
    .split(/[.-]/)
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}
```

Used by: `session-info-panel.tsx`, `ModelSelect.tsx`, and the CLI parameter settings page.

### 7. Discovery integration point

**Decision:** `chat_profile.rs`'s `defaults_for()` calls the native config port before falling back to `default_model_for_agent()`. The precedence is:

1. Explicitly persisted session chat config → no discovery needed
2. VaneHub CLI profile `model` parameter value (if set and not "default")
3. Native config discovered model
4. Hardcoded agent default model

```rust
fn defaults_for(&self, agent_id: &str) -> Result<ChatConfigurationValues> {
    let selections = self.cli_parameters.load_selections(agent_id)?;
    let model_id = selections.get("model")
        .and_then(Value::as_str)
        .and_then(|model| model_id_from_cli(agent_id, model))
        .or_else(|| self.native_config.discover_model(agent_id).ok().flatten()
            .and_then(|raw| model_id_from_cli(agent_id, &raw)))
        .unwrap_or_else(|| default_model_for_agent(agent_id));
    // ...
}
```

### 8. Web/mock adapter parity

**Decision:** `web-agent-client.ts`'s `getSessionChatConfig()` returns a mock `ChatConfig` with the hardcoded default model. No native config files exist in browser context. A future HTTP backend could provide its own discovery.

### 9. Claude Code per-project usage-cache fallback (`~/.claude.json`)

**Decision:** When `~/.claude/settings.json` has no `ANTHROPIC_MODEL`, fall back to Claude Code's own per-project state file `~/.claude.json` → `projects[normalized_workspace_path].lastModelUsage`. Only trust this source when the `lastModelUsage` object has **exactly one key** — treat two or more (a project with mixed model history) as unknown, since there is no timestamp field to determine which was used most recently.

**Trigger:** Discovered while dogfooding — a real Claude Code session showed `deepseek-v4-pro` in its own banner (set via the CLI's own runtime, outside any file VaneHub previously read), while VaneHub's info panel still showed the hardcoded default `claude-opus-4-8`. Neither `~/.claude/settings.json` nor VaneHub's own CLI parameter profile had a model value — the only place the model appeared was Claude Code's internal per-project usage cache.

**Path normalization:** Project keys in `~/.claude.json` use forward slashes (e.g. `D:/cdavid/Documents/code/gemini-cli`) regardless of OS. `normalize_project_path()` lowercases and converts backslashes to forward slashes before comparing, so Windows-style session workspace paths (`D:\cdavid\...`) still match.

**Rationale:** `lastModelUsage` is the only per-project model signal Claude Code persists locally. Restricting to the single-key case avoids guessing when history is ambiguous — the existing hardcoded-default fallback is a safer answer than a wrong guess.

**Explicitly accepted risk:** `~/.claude.json` is Claude Code's internal, undocumented state file (unlike `settings.json`, which is publicly documented). Its structure may change across Claude Code versions without notice, silently disabling this fallback (degrading gracefully to the next fallback in the chain, not erroring). This is a deliberate scope extension beyond the original "read documented config" design — accepted because it fixes a real, reproducible gap, and the failure mode is silent degradation rather than breakage.

**Scope:** Initially scoped to Claude Code only. Auditing the other three CLIs' local state (see Decisions 10 and 11) found the same class of gap in Codex CLI and OpenCode; Gemini CLI showed no equivalent evidence on inspection (empty `settings.json`, no per-project state beyond shell-history directory markers) and was left unchanged.

### 10. Codex CLI project-scoped `model` override in `config.toml`

**Decision:** Before reading the top-level `model` key, check `config.toml`'s `[projects.'<path>']` table (the same table Codex CLI uses for `trust_level`) for a project-scoped `model` override, using the same `normalize_project_path()` matching as Decision 9.

**Trigger:** Auditing Codex CLI's config after the Claude Code fix — `~/.codex/config.toml` on the dogfooding machine has 11 `[projects.'<path>']` entries (Codex CLI's own per-project trust registry). The schema documented by Codex CLI supports a `model` key at that level, alongside `trust_level`; VaneHub's discovery only ever read the file's top-level `model`, silently missing any project-scoped override.

**Rationale:** Unlike Decision 9, this reads the *same documented file* VaneHub already parses — no new file, no undocumented internal format, and no ambiguity to resolve (a project section either has a `model` key or it doesn't; there's no multi-value history to disambiguate). This is a strictly more complete read of an already-in-scope source, not a scope extension in the way Decision 9 was.

**Precedence:** VaneHub CLI profile model → `config.toml` project-scoped `model` → `config.toml` top-level `model` → hardcoded default.

### 11. OpenCode active model lives in `opencode.db`, not `opencode.json`

**Decision:** Before falling back to the static `opencode.json` catalog, query OpenCode's own SQLite state at `~/.local/share/opencode/opencode.db` — table `session`, columns `directory` and `model` (JSON `{"id": ..., "providerID": ...}`), ordered by `time_updated` — for the most recent session matching the workspace path (via `normalize_project_path()`). Opened read-only (`SQLITE_OPEN_READ_ONLY | SQLITE_OPEN_NO_MUTEX`) with a short busy timeout (200ms); any failure (missing file, lock contention, malformed row) yields `None` and falls through to the config-file reading that already existed.

**Trigger:** Auditing OpenCode's local state — `opencode.json` only declares which models a *provider* makes available (`provider.<id>.models`), it is not a record of what's actually selected. The original discovery implementation picked the first model key from the first provider, which has no relationship to what the user is running; real session data (queried directly from the dogfooding machine) showed sessions scoped to specific project directories consistently using a specific model (e.g. `deepseek-v4-flash`) that never appeared anywhere in the static config.

**Rationale:** This is a more reliable signal than Decision 9's Claude Code fallback — `time_updated` gives an unambiguous "most recent" ordering, so there's no multi-value guessing problem. `rusqlite` is already a VaneHub dependency (used for its own storage), so no new crate was needed. The two-query approach (distinct directories, then a targeted `directory = ?` lookup) avoids scanning the full session history and avoids relying on SQLite collation for case/separator-insensitive path matching.

**Explicitly accepted risk:** This reads another application's live SQLite database file, which may be open with active WAL/SHM files from a running OpenCode process. Read-only + short busy-timeout + treating any error as "not found" keeps this from ever blocking VaneHub's own session creation; worst case is silent fallback to the pre-existing config-file behavior.

**Precedence:** VaneHub CLI profile model → `opencode.db` session lookup for the workspace directory → `opencode.json` first provider's first model → hardcoded default.

## Risks / Trade-offs

- **[Risk] Breaking change if a user's CLI native config has a model VaneHub previously rejected** → Mitigation: the passthrough is strictly additive; VaneHub no longer blocks models, it just passes them through. Existing sessions with persisted configs are unaffected.
- **[Risk] `model_id_from_cli` return type change (`&'static str` → `String`)** → Mitigation: This is an internal function used only in two places (`chat_profile.rs` and tests). All callers already `.map(str::to_string)` the result. The allocation is negligible.
- **[Risk] OpenCode JSON5 parsing complexity** → Mitigation: For OpenCode model discovery, we use a simple regex or string search to extract `provider.<id>.models` keys rather than a full JSON5 parser. If the file is unparseable, we fall back to the default. A full json5 parser dependency is not warranted for extracting one field.
- **[Risk] Native config read failures on every session load** → Mitigation: Discovery failures are logged as diagnostics and never surfaced to the user. They don't block session creation.
- **[Risk] `CustomText` control might confuse users who think any string will work** → Mitigation: The free-text field includes a description noting "Enter any model identifier your CLI supports" or similar i18n text.
- **[Risk] `~/.claude.json` structure changes across Claude Code versions** → Mitigation: parsed as loose `serde_json::Value` (not a typed struct), any missing/renamed field yields `None` and falls through to the hardcoded default; failures are logged as diagnostics, never surfaced to the user. See Decision 9.

## Open Questions

- Should native config discovery be cached within a session lifetime, or re-read on every config query? Leaning: re-read on session start only; the info panel's `useQuery` already caches via React Query.
- Should the model selector dropdown show the custom model as a separate visual style (e.g., italic or with a "custom" badge)? Leaning: yes, to distinguish known/supported models from user-entered ones.
