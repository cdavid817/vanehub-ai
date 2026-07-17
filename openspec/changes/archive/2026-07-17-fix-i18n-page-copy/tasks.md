## 1. Audit and Terminology

- [x] 1.1 Scan `src/**/*.tsx` and frontend-owned `src/services/**/*.ts` for user-visible hard-coded Chinese and English strings.
- [x] 1.2 Classify findings into translatable UI copy versus allowed stable identifiers such as product names, provider names, package names, executable names, model names, paths, command strings, and ids.
- [x] 1.3 Define a small zh-CN/en terminology map for Agent, Skill, CLI, SDK, MCP, workspace, session, install, update, rollback, upgrade, downgrade, refresh, and operation logs.

## 2. Locale Resources

- [x] 2.1 Add missing zh-CN and en keys for Agents settings page labels, filters, details, notices, launch actions, and error/empty states.
- [x] 2.2 Add missing zh-CN and en keys for SDK Dependencies page labels, status values, stat cards, actions, confirmations, notices, errors, empty states, and operation logs.
- [x] 2.3 Add missing zh-CN and en keys for MCP Servers page, MCP server cards, forms, import/export modal, validations, confirmations, notices, scope labels, and empty states.
- [x] 2.4 Add missing zh-CN and en keys for create-session dialog, workspace information tabs, sidebar/context actions, status bar helper text, and date/time labels.
- [x] 2.5 Add missing zh-CN and en keys for chat configuration selectors, role labels, button titles, permission/mode descriptions, and frontend-owned chat helper text.
- [x] 2.6 Review existing zh-CN/en pairs and correct semantically mismatched or inconsistent translations.

## 3. Component Updates

- [x] 3.1 Replace hard-coded user-visible text in `src/settings/pages/agents-page.tsx` with translation keys.
- [x] 3.2 Replace hard-coded user-visible text in `src/settings/pages/sdk-page.tsx` with translation keys.
- [x] 3.3 Replace hard-coded user-visible text in `src/settings/pages/mcp-page.tsx` and `src/settings/pages/mcp/*` with translation keys.
- [x] 3.4 Replace hard-coded user-visible text in `src/main-layout/create-session-dialog.tsx`, workspace layout modules, information panel tabs, and status/date formatting with translation keys and active-language formatting.
- [x] 3.5 Replace hard-coded user-visible text in chat selector/configuration components and message role/timestamp formatting with translation keys and active-language formatting.
- [x] 3.6 Localize frontend Web/mock user-facing errors or wrapper messages that are displayed directly in UI while preserving stable identifiers.

## 4. Project Standards

- [x] 4.1 Create or update `openspec/project.md` with a frontend i18n rule requiring zh-CN and en support for all new or changed user-visible page text.
- [x] 4.2 Document allowed literal exceptions for stable identifiers, package names, commands, paths, protocol names, model names, and provider/product names.
- [x] 4.3 Ensure AGENTS.md guidance remains consistent with the project-level i18n standard.

## 5. Tests and Guardrails

- [x] 5.1 Keep or update the existing i18n resource parity test so zh-CN and en key sets must match.
- [x] 5.2 Add a focused hard-coded user-visible text guardrail test or script for page/shared UI components with an explicit allowlist.
- [x] 5.3 Update settings, workspace, chat, SDK, MCP, and Agents tests whose assertions currently depend on hard-coded Chinese or English literals.
- [x] 5.4 Add targeted tests or snapshots proving representative pages render correct Chinese and English text after language changes.

## 6. Verification

- [x] 6.1 Run `npm run test`.
- [x] 6.2 Run `npm run build`.
- [x] 6.3 Run `cargo test --manifest-path src-tauri/Cargo.toml` if implementation touches native code.
- [x] 6.4 Run `cargo check --manifest-path src-tauri/Cargo.toml` if implementation touches native code.
- [x] 6.5 Run `openspec validate --specs --strict`.
- [x] 6.6 Run `openspec validate fix-i18n-page-copy --strict`.
