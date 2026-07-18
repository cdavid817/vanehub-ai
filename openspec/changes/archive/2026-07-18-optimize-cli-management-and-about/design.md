## Context

The current CLI Management page already reads cached CLI status, supports asynchronous refresh, shows installation conflicts, exposes single-tool package actions, and keeps React behind the agent service boundary. The About page already has a compact CLI environment summary. The remaining gap is a cc-switch-like local environment check workflow where users can refresh, diagnose, and upgrade all eligible managed CLIs from the same operational surface.

## Goals / Non-Goals

**Goals:**

- Add a bulk upgrade path for the four managed CLI stable ids: `claude-code`, `codex-cli`, `gemini-cli`, and `opencode`.
- Keep package mutation isolated per CLI and represented through backend-managed operations with card-local logs where possible.
- Keep desktop and Web/mock behavior behind the same `AgentService` contract.
- Keep the About page summary compact and navigational, not a second lifecycle-control surface.
- Preserve both `futuristic` and `minimal` visual styles through existing semantic tokens.

**Non-Goals:**

- Adding new supported CLI tools beyond the existing four.
- Adding WSL shell selection or per-shell command overrides.
- Performing host CLI inspection in Web/mock mode.

## Decisions

1. Add `upgradeAllCliVersions()` to the frontend `AgentService`.

The bulk action crosses runtime boundaries and starts native work, so it belongs in the service interface and adapters rather than being assembled directly in React. The Tauri adapter maps to a new command, while the Web adapter returns the existing localized unsupported operation shape.

Alternative considered: call `installCliVersion()` repeatedly from the React page. That would leak sequencing policy and preflight edge cases into UI state and make logs look like unrelated operations.

2. Implement desktop bulk upgrade as one user-visible asynchronous native operation with per-CLI worker isolation.

The operation will inspect cached statuses, select tools that have a backend-managed lifecycle and have a latest version newer than the current version, then run each eligible CLI in its own worker. A per-CLI mutation guard prevents two operations from mutating the same CLI at once while allowing different CLIs to proceed independently.

Alternative considered: keep one global CLI mutation guard. That is simpler, but it makes one slow or failed CLI block unrelated CLI upgrades.

3. Keep conflict confirmation for single-tool mutations only in the first iteration.

The existing single-tool action already requires confirmation when multiple installations exist. For bulk upgrade, tools with multiple installations or unsupported active sources will be skipped and logged with guidance, preventing the batch from silently mutating an ambiguous target.

Alternative considered: show a multi-tool confirmation dialog. That is a larger interaction change and can follow once the basic bulk operation exists.

4. Polish About using the existing environment summary data.

About should show installed and attention counts plus navigation to CLI Management. It should not duplicate refresh, diagnose, install, or upgrade controls because those actions already require detailed per-tool context and logs.

5. Split manual guidance by detected cause.

cc-switch avoids one generic "manual" warning by separating broken executables, multiple installations, and source-specific update paths. VaneHub will keep mutations limited to verified backend-managed sources, and the card guidance will use the detected active installation and conflict state to explain the exact reason: check environment for broken CLIs, inspect diagnostics for multiple installations, or use the source-native updater for unsupported active paths.

Alternative considered: port cc-switch's source-specific anchored command generator. That is useful long term, but it expands the native mutation surface beyond the current npm-only safety boundary and should be proposed separately.

6. Add a small source-aware lifecycle planner instead of keeping npm as the only executable mutation path.

The planner selects npm for active npm installs, wget-based official scripts for active vendor/script installs when the tool has an official script URL, WinGet for active WinGet installs when the tool has a verified WinGet package id, and wget-based scripts for first installs when available. Tools without an official script continue to use npm for first install. This keeps upgrades aligned with the old installation source and avoids adding a second npm installation over a source-native CLI.

For now, wget-script execution means a backend-owned `bash -lc` command that prefers `wget` and falls back to `curl` only when `wget` is unavailable. WinGet execution means a backend-owned `winget upgrade --id <verified-id> --exact` command. Unsupported sources such as unknown, system, desktop, homebrew, volta, or bun still require manual/source-native handling until each source has a verified safe command plan.

## Risks / Trade-offs

- Bulk operation partially succeeds -> logs include each skipped, failed, and upgraded tool; final status remains successful only when the operation itself completes and refreshed statuses are persisted.
- Cached latest version may be stale -> users can refresh first; the bulk operation also re-detects each processed tool after package mutation.
- Web users may see an action that cannot run locally -> Web adapter returns a localized unsupported operation and About/CLI pages continue to avoid implying host inspection.
- More specific guidance may still require user judgment -> each message points to diagnostics, current source, or environment checks instead of implying VaneHub can repair every source.
- Wget-script installs require a local `bash` plus either `wget` or `curl`; failures are logged through the existing operation and diagnostic log path without silently falling back during source-preserving upgrades.
