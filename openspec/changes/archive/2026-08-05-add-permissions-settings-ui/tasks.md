## 1. Backend — non-mutating principal read (D1)

- [x] 1.1 Add `PermissionsApi::find_principal(agent_id: &str) -> Option<Principal>` calling `PrincipalRepository::find_by_agent_id` directly — no `get_or_create_principal`, no write
- [x] 1.2 Add `EvaluationService`/`PermissionsApi` logic to synthesize the effective `PrincipalEntry` when no row exists: template = current default (via `DefaultTemplatePort`, see Group 2), `requiresConfirmationToAssign` computed the same way as a real row
- [x] 1.3 Unit tests: existing row round-trips unchanged; absent row synthesizes the current default without writing anything; changing the default between two synthesized reads is reflected (no caching)

## 2. Backend — configurable default template (D2)

- [x] 2.1 Add `DefaultTemplatePort` trait (`fn default_template(&self) -> PolicyTemplateName`) to `permissions::application::ports`
- [x] 2.2 Add `DesktopDefaultTemplateAdapter` in `permissions::infrastructure`, implementing the port by calling `crate::contexts::desktop::api` (never `desktop::domain` directly)
- [x] 2.3 Add `DesktopSettingKey::DefaultPolicyTemplate` variant, schema/repository field, and struct field in `contexts/desktop/domain/settings.rs`, following the exact pattern `automaticArchivalEnabled` already uses
- [x] 2.4 Wire the adapter in `bootstrap/permissions.rs`; update `EvaluationService::get_or_create_principal` to read the port instead of hardcoding `PolicyTemplateName::Standard`, falling back to `Standard` on any port failure
- [x] 2.5 Unit tests: new agent inherits the configured default; missing/corrupt setting falls back to `standard`; changing the default after a principal already exists does not retroactively change it
- [x] 2.6 Run `cargo test` (not `--lib`) to confirm the architecture suite accepts the new port/adapter split — do not substitute a partial test run

## 3. Backend — Tauri command

- [x] 3.1 Add `get_agent_policy_principal` command (`src-tauri/src/commands/permissions/`) returning `PrincipalEntry` for a given `agentId`, using `find_principal` (Group 1) — a pure delegation with no branching in the command function itself (match this project's zero-control-flow command-adapter convention; put any bool/enum mapping in `mapper.rs` if needed)
- [x] 3.2 Register the command in `commands/registry.rs`
- [x] 3.3 Add `DesktopSettingKey::DefaultPolicyTemplate` to whatever existing desktop-settings command(s) already read/write settings (get/update) — no new command needed if the existing settings command is generic over keys

## 4. Frontend — service layer

- [x] 4.1 Add `getAgentPolicyPrincipal(agentId: string): Promise<PrincipalEntry>` to `src/services/permissions.ts`
- [x] 4.2 Implement in `tauri-permissions-client.ts` (invoke `get_agent_policy_principal`)
- [x] 4.3 Implement in `web-permissions-client.ts` using `web-permissions-mock-state.ts`'s existing `webPrincipalTemplates` map, synthesizing the default template the same way the backend does (mirror Group 1.2's logic in mock form) — the shared default itself lives in a new `webDefaultPolicyTemplate` getter/setter pair in `web-permissions-mock-state.ts`, kept in sync by `web-settings-client.ts`'s `saveSetting`
- [x] 4.4 Confirm `runtime-permissions-client.ts` needs no change (adapter selection only)
- [x] 4.5 Add the default-policy-template field to the desktop settings type/service (wherever `AppSettings`/`DesktopSettingKey`-equivalent frontend types live) and both its Tauri and Web/mock client implementations — both clients needed zero code changes since `saveSetting`/`getSettings` are already fully generic over `AppSettingKey`; only `types/settings.ts`, `settings-service.ts` (default value + validation), and `settings-provider.tsx`'s hardcoded `resettableKeys` list needed updating

## 5. Frontend — new settings page

- [x] 5.1 Create `src/settings/pages/agent-policies-page.tsx`: fetch `listAgents()`, filter to `agentOrigin === "user"` union `id === "onepiece"` (D4), fetch each agent's principal via `getAgentPolicyPrincipal`, render one row per agent with a template picker (readonly/standard/trusted/yolo)
- [x] 5.2 Implement confirm-to-increase-trust (D3): before calling `applyPolicyTemplate` with `trusted`/`yolo`, check the target template's `requiresConfirmationToAssign` (already returned by the DTO) and show a confirmation dialog describing unattended shell/file access; apply immediately for `standard`/`readonly`
- [x] 5.3 Register the page in `src/settings/settings-pages.ts` (new `agent-policies` id, lazy loader, following the existing entry pattern)
- [x] 5.4 Add the global default-template picker to `src/settings/pages/basic-settings-page.tsx`, wired to the field added in 4.5 — reused the existing `SelectField` generic component already used for language/theme/fontSize, not a new picker widget
- [x] 5.5 Visual pass: futuristic/minimal semantic tokens, 8px radius, no nested cards — matched existing `PageHeader`/`SectionPanel`/`SettingsRow` primitives and `ApprovalCard`'s established button-group pattern for the template picker; no new visual language introduced

## 6. i18n

- [x] 6.1 Add new keys (page title/description, column headers, template names/descriptions, confirm-to-increase-trust dialog copy, default-template setting label) to `en.json` as the canonical source
- [x] 6.2 Mirror into `zh-CN.json`, `zh-TW.json`, `ja.json`, `ko.json` with matching keys/interpolations
- [x] 6.3 Run the i18n resource-parity test to confirm exact key/interpolation parity across all 5 locales

## 7. Verification

- [x] 7.1 `cargo check --manifest-path src-tauri/Cargo.toml`
- [x] 7.2 `cargo test --manifest-path src-tauri/Cargo.toml` (full suite, including `tests/architecture.rs` — see 2.6) — 1293 passed, 0 failed, 9 ignored
- [x] 7.3 `cargo clippy --manifest-path src-tauri/Cargo.toml` — clean (only the pre-existing, intentionally-reserved dead-code warnings from `add-permissions-core` remain)
- [x] 7.4 `npm run lint`
- [x] 7.5 `npm run test` — 527 passed (two pre-existing hardcoded settings-page-count assertions updated from 14 to 15)
- [x] 7.6 `npm run build` — 16 lazy chunks verified, main static closure within budget
- [x] 7.7 `openspec validate add-permissions-settings-ui --strict`
- [x] 7.8 Visual pass, automated via a scripted headless-Chromium session against `npm run dev` (Web/mock adapter, not the native Tauri client — no driver tooling was available for the native WebView2 window, but the React component tree is identical across both runtimes): confirmed the nav entry lands in the right place, the page title/description/list render with correct i18n text, OnePiece appears in the list at the `standard` default with all four template buttons, clicking `信任` (Trusted) opens the confirm-to-increase-trust dialog with the correct copy, confirming applies the change and the UI reactively shows `信任` as the now-selected template (`aria-pressed="true"`), and the Basic Settings page's new default-template picker renders with all four options — zero console errors across the whole flow
