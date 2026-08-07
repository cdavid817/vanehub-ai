## Why

`add-permissions-core` shipped the full backend Policy Decision Point — four policy templates (readonly/standard/trusted/yolo), a `PrincipalRepository`, and an `apply_policy_template` Tauri command — but has zero frontend consumers. There is no page anywhere in the product that lists a custom API agent's current template, and no way to change it short of editing SQLite by hand. This was a known, explicitly documented scope cut in `add-permissions-core`'s own `tasks.md` (8.2/8.3/8.5): no agent-management UI existed to host a template picker, so the picker itself was deferred. This change builds that missing surface.

## What Changes

- **BREAKING**: none — this is purely additive; no existing command, route, or UI element is removed or renamed.
- Add a read-side `PermissionsApi` capability (`get_agent_policy_principal` command) that returns a `PrincipalEntry` for a given `agent_id` without mutating anything — the only read path today (`assign_template`) always writes.
- Add a new Settings page listing every custom API agent (`AgentRegistryEntry.agentOrigin === "user"`) plus the built-in OnePiece agent, each row showing its current policy template and a picker to change it (readonly/standard/trusted/yolo).
- Reuse the already-computed `PrincipalEntry.requiresConfirmationToAssign` field to gate a confirmation step when raising an agent to `trusted`/`yolo` — this is `add-permissions-core` task 8.3/8.5's deferred confirm-to-increase-trust interaction, completed here now that a UI exists to host it.
- Add a global "default template for newly created agents" desktop setting (new `DesktopSettingKey` variant, a picker in `basic-settings-page.tsx`), consumed by `EvaluationService::get_or_create_principal` in place of its current hardcoded `PolicyTemplateName::Standard`, with `Standard` kept as the fallback when the setting is absent or unreadable.
- Add the corresponding frontend service method (`permissions.ts` + all three client implementations) and i18n strings across all 5 locales.

## Capabilities

### New Capabilities

(none — this extends capabilities `add-permissions-core` already introduced)

### Modified Capabilities

- `permissions-approval`: adds the template-picker UI, the read-side principal lookup it depends on, and the confirm-to-increase-trust interaction that capability's own requirements already anticipated but left unimplemented pending a UI surface.
- `permissions-core`: adds a configurable default template for newly created principals, replacing the fixed `Standard` default with "read the desktop setting, fall back to `Standard`."

**Sequencing note**: `add-permissions-core` has not been archived yet (52/59 tasks; see `design.md` for exactly what's outstanding). Its `permissions-approval`/`permissions-core` spec deltas therefore do not yet exist in `openspec/specs/`. This proposal's own deltas are written as continuations of those pending capabilities and cannot be archived before `add-permissions-core` is — see `design.md`'s Dependencies section.

## Impact

- **Runtime**: desktop only for the new settings UI and the desktop-setting storage (Tauri command + SQLite-backed `desktop` settings store); no Web-runtime-only surface, but the Web/mock adapter must implement full parity per this project's existing pattern (`web-permissions-client.ts`, `web-agent-client.ts`, mock desktop settings store).
- **Affected code**:
  - `src-tauri/src/contexts/permissions/` (new read method on `EvaluationService`/`PermissionsApi`)
  - `src-tauri/src/commands/permissions/` (new command + DTO + mapper entry)
  - `src-tauri/src/contexts/desktop/` (new `DesktopSettingKey` variant, schema/repository field)
  - `src/settings/pages/` (new page) and `src/settings/settings-pages.ts` (registration) and `src/settings/pages/basic-settings-page.tsx` (new default-template picker)
  - `src/services/permissions.ts` + `tauri-permissions-client.ts` + `web-permissions-client.ts` + `runtime-permissions-client.ts` (new read method)
  - `src/services/settings.ts`-equivalent desktop settings service and its client implementations (new field)
  - `src/i18n/locales/{zh-CN,en,zh-TW,ja,ko}.json`
- **Out of scope**: a form to create new custom API agents (a pre-existing, unrelated gap — `registerApiAgent` has zero frontend callers today); any CLI agent (Claude Code/Gemini CLI/OpenCode/Codex CLI) policy configuration (those agents are not wired into `permissions` at all yet — that is Phase 2/3's scope); the `permission://request` push event and notification-system wiring (`add-permissions-core` tasks 4.5/4.6, still outstanding on that change, not this one).
- **Dependencies**: `add-permissions-core` must be archived before this change can be archived (OpenSpec governance: a phase's change is only opened once its predecessor is archived — this proposal is being written ahead of that per explicit user direction, with the dependency called out rather than assumed satisfied).
