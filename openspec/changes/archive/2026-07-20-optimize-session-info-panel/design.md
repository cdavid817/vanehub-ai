## Context

`src/main-layout/session-info-panel.tsx` currently renders keep-alive tabs for Basic Info, Files, Changes, and Logs. Basic Info contains an earlier hard-coded progress summary and a small session configuration block; Files, Changes, and Logs duplicate information that already has richer workspace tabs. The requested panel should instead answer three questions for the active CLI session: what is this session running, how much usage has this session consumed, and which Skills are relevant to the selected CLI and project.

Usage records already persist `session_id`, reported token counts, estimated character counts, provider/model metadata, and occurrence time in SQLite. Existing `getUsageStatistics` aggregates by time range for the settings page, but there is no frontend service method for a single-session summary. Skills already have global/workspace scopes, enabled state, Agent bindings, and mount metadata, so the panel can reuse `listSkills` instead of adding Skill-domain behavior.

## Goals / Non-Goals

**Goals:**
- Replace the right information panel with Basic Info, Token Usage, and Skill tabs while preserving collapse, keep-alive tab state, and internal scrolling.
- Show the model id from `getSessionChatConfig(activeSession.id)` as the source of truth for the selected large model.
- Add a session-scoped usage summary contract that prefers reported token totals and exposes estimated fallback context separately.
- Show available Skills for the active CLI and project Skills for the active workspace, with disabled project Skills visually de-emphasized and excluded from the available group.
- Keep all UI text in zh-CN/en i18n resources and all styling on existing semantic theme tokens.

**Non-Goals:**
- Do not add provider billing, monetary cost estimation, provider/model filtering, or external CLI history scanning.
- Do not redesign the Settings usage statistics page.
- Do not change Skill creation, import, binding, mount migration, or drift synchronization behavior.
- Do not infer a model from rendered CLI argv; the panel uses session chat configuration `modelId`.
- Do not add theme-specific React branches for `futuristic` or `minimal`.

## Decisions

1. Add a session usage service method instead of deriving totals only from loaded messages.

   The frontend can technically sum `ChatMessage.tokenUsage`, but the workspace only loads a bounded message page and message usage does not expose cache-read/cache-creation or estimated coverage. A service method such as `getSessionUsageSummary(sessionId)` should aggregate persisted `usage_records` for exactly one session in the native runtime and return a deterministic Web/mock equivalent. This keeps accounting consistent with the existing usage system.

2. Reuse the existing usage totals shape where possible.

   The new response should carry reported token totals, estimated character totals, coverage counts, and `generatedAt`. It does not need daily trends, by-Agent breakdowns, or time-range filtering because the panel is scoped to one active session and should remain compact.

3. Use `getSessionChatConfig` for the Basic Info model.

   The panel should display `modelId` from session chat configuration. If no explicit `modelId` is present, it should show a localized empty or default-unavailable state rather than reconstructing CLI argument mapping in the UI. CLI display remains based on stable `activeSession.agentId` and existing visual identity helpers.

4. Compose Skill groups from existing Skill APIs.

   The panel should query global Skills and workspace Skills when the active session has `worktreePath` or `projectPath`. Available Skills are enabled Skills whose `boundAgentIds` include the active `agentId` or whose mounted binding for that Agent is true. Project Skills are the workspace-scope Skills for the active workspace, shown separately; disabled project Skills remain visible in that group with muted styling but are not counted as available.

5. Keep the panel presentational and service-backed.

   `SessionInfoPanel` may use React Query and `agentService`, matching the existing component pattern. It must not import Tauri APIs. Any new native command call belongs in `tauri-agent-client.ts`; the Web adapter must expose the same method with compatible mock data.

6. Preserve theme and layout behavior through shared classes.

   The implementation should continue using `ucd-panel`, `ucd-muted-panel`, `ucd-segmented`, semantic text/background/border classes, and lucide icons. Avoid explicit checks for the active theme so both registered styles stay aligned with the design system.

## Risks / Trade-offs

- [Risk] Adding a session usage API touches Rust, TypeScript contracts, Tauri adapter, and Web adapter for a compact panel feature. -> Mitigation: keep the response shape narrow and reuse existing usage aggregation models and SQL helpers where practical.
- [Risk] Skill data may load more slowly than the panel shell. -> Mitigation: render per-tab loading/empty/error states while preserving already loaded panel content.
- [Risk] Workspace Skill scope selection can be ambiguous when both `worktreePath` and `projectPath` exist. -> Mitigation: prefer `worktreePath` because it is the active working tree for the session, then fall back to `projectPath`.
- [Risk] A three-tab segmented control with translated labels can overflow at 300px panel width. -> Mitigation: use stable grid columns, compact text, truncation, and accessible titles/labels.

## Migration Plan

No database migration is required because existing usage records already include `session_id`. The change is additive at the command/service-contract level and can be rolled back by removing the new session usage method and restoring the previous tab content. Existing global usage statistics behavior remains unchanged.

## Open Questions

None. Product decisions are resolved: reported tokens are primary, estimated activity is fallback context, model source is session chat config `modelId`, and Skills are grouped as available versus project Skills.
