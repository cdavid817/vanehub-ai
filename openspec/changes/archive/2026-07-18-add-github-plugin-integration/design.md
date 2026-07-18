## Context

The settings center already has service-backed pages for SDK dependencies, local extension capabilities, MCP servers, Agents, Skills, IM, and usage statistics. There is no product-level Plugin Integrations page, and "plugin" currently overlaps with Skills, MCP servers, and local extensions without a unified entry point.

`clowder-ai` has a broader declarative plugin model and a built-in GitHub plugin. For VaneHub, adopting the whole manifest/runtime system now would introduce security, resource activation, and permission decisions that are larger than the requested GitHub integration. The first version should establish the settings surface and service boundary while keeping executable behavior narrow.

## Goals / Non-Goals

**Goals:**
- Add a Plugin Integrations settings page with a built-in GitHub plugin card.
- Route React through a `PluginIntegrationService` with Tauri and Web/mock adapters.
- Let the desktop backend list the built-in catalog and test GitHub readiness through `gh auth status`.
- Show deterministic, honest Web/mock behavior without claiming live host inspection.
- Keep visual styling consistent with both `futuristic` and `minimal` settings styles.
- Keep all user-visible text in synchronized zh-CN and en locale resources.

**Non-Goals:**
- No third-party plugin installation, upload, uninstall, or marketplace.
- No `plugin.yaml` manifest parser or plugin resource activation.
- No PAT persistence, OAuth device flow, GitHub App flow, or browser login callback.
- No PR, Issue, CI, or Review automation in the first version.
- No direct React calls to Tauri `invoke()`.

## Decisions

1. Add a dedicated plugin integration service instead of extending `AgentService`.

   GitHub plugin readiness is product integration state, not an AI agent lifecycle or session concern. A dedicated `src/services/plugin-integration-service.ts` keeps the contract focused and allows future plugin integrations to grow without bloating the agent service. The Tauri adapter will call declared native commands; the Web adapter will return deterministic data and desktop-only test limitations.

2. Model GitHub as a built-in integration, not a user-installed plugin.

   The first version needs one known integration with stable id `github`. This avoids manifest parsing, arbitrary resource paths, external command definitions, and third-party permission models. It also matches VaneHub's existing pattern for built-in local extension definitions.

3. Use `gh auth status` for readiness and do not store credentials.

   Shelling out to the GitHub CLI lets users rely on the official GitHub authentication flow and avoids adding plaintext or encrypted token storage in this change. The backend will only execute a backend-owned command plan and return concise status. If `gh` is missing or unauthenticated, the UI will show setup guidance.

4. Treat GitHub checks as observable native work.

   A connection test may start an external command and can fail due to missing executable, timeout, network, or authentication state. The service should expose loading/running/terminal feedback through the existing frontend fetching/mutation pattern. Native diagnostics should be persisted through unified logging with redaction.

5. Reuse settings primitives and semantic tokens.

   The page should use `PageHeader`, `SectionPanel`, `StatCard`, `StatusPill`, shared buttons, lucide icons, and `ucd-panel` styling. No page-specific palette, nested card decoration, or theme-name branching is needed.

## Risks / Trade-offs

- [Risk] Users may expect "plugin" to mean installable third-party packages. -> Mitigation: label the page as built-in integrations in copy and explicitly defer marketplace/install behavior in specs.
- [Risk] `gh auth status` can include account, host, or path details. -> Mitigation: return concise normalized status to the frontend and write only redacted diagnostics through unified logging.
- [Risk] GitHub CLI might be missing even when users have Git installed. -> Mitigation: represent `missing_cli` separately and include localized setup steps with an official docs link.
- [Risk] Web/mock mode may appear to test real GitHub state. -> Mitigation: Web adapter always reports live checks unavailable and uses deterministic mock status.
- [Risk] Navigation ordering can crowd settings. -> Mitigation: place Plugin Integrations between Extension Capabilities and MCP Servers, near adjacent integration surfaces.

## Migration Plan

1. Add delta specs and validate the OpenSpec change.
2. Implement TypeScript types, service interface, Tauri/Web adapters, and the settings page.
3. Add Rust command module and register commands for listing integrations and testing GitHub readiness.
4. Add locale keys and tests for page registration, filtering/status behavior, adapter parity, and i18n parity.
5. Rollback by removing the settings page entry, service adapters, native commands, and locale keys. No user credential data or database migration is introduced in the first version.

## Open Questions

- Should a future version support optional PAT storage, and if so should it use SQLite plus OS credential storage instead of plaintext fields?
- Which GitHub workflows should be next after readiness: repository status, PR review inbox, issue creation, CI monitoring, or MCP GitHub server bootstrap?
