## Context

The app already initializes i18next and has zh-CN/en locale files with key parity tests. Basic settings, CLI management, Skills, and parts of the workspace use `useTranslation`, but several active pages still render hard-coded English or Chinese text. Examples found during proposal research include create-session dialog Chinese literals, Agents page English labels and notices, SDK page English labels and statuses, MCP page English labels and confirmations, chat selector labels, fixed `zh-CN` date formatting, and Web/mock user-facing errors.

Existing `settings-center-ui` only requires localized text for settings center pages. The new requirement needs to cover all React user-visible surfaces and project-level future-work rules.

## Goals / Non-Goals

**Goals:**

- Make all page-level user-visible copy render through synchronized zh-CN/en translation resources.
- Correct mistranslated or semantically inconsistent zh-CN/en values where existing keys diverge.
- Add missing keys for settings pages, workspace layout/dialogs, chat selectors, notices, confirmations, empty states, and frontend Web/mock errors shown to users.
- Use locale-aware formatting for dates/times based on the active i18n language.
- Add project-level i18n rules so future UI/page changes include both zh-CN and en resources.

**Non-Goals:**

- Translating product names, provider names, npm package names, executable names, stable ids, model names, file paths, command strings, log levels, or protocol names where they are identifiers.
- Localizing backend diagnostic log content unless that content is directly displayed as user-facing UI copy.
- Adding new languages beyond zh-CN and en.
- Introducing a new i18n library or replacing i18next.

## Decisions

1. Keep i18next as the single frontend localization mechanism.

   React components should use `react-i18next` and keys under `src/i18n/locales/zh-CN.json` and `src/i18n/locales/en.json`. This matches existing app initialization and avoids a second localization path. The alternative of component-local dictionaries would make parity and future audits harder.

2. Treat user-visible UI text as translatable by default.

   Page titles, descriptions, button labels, placeholders, badges, status labels, notices, confirmations, modal labels, empty states, tooltip titles, aria-labels that contain readable text, and frontend mock/runtime errors displayed to users should use translation keys. Stable product/domain identifiers remain literal.

3. Prefer feature-scoped key namespaces.

   Existing namespaces such as `basic`, `cli`, `skills`, `layout`, and `chat` should be extended. New or incomplete surfaces should use predictable prefixes like `agents`, `sdk`, `mcp`, `createSession`, and `chat.config`. This keeps large locale files navigable without adding dependencies.

4. Add automated guardrails in tests.

   The existing key parity test should remain. Implementation should add a focused hard-coded text audit test or script for page/component files with an allowlist for identifiers. This catches regressions when future page changes add visible literals without translations.

5. Put the long-term rule in project standards.

   Because `openspec/project.md` is referenced by AGENTS but currently absent in this checkout, implementation should create or update the project standards document with the i18n contract. The standards should explicitly require zh-CN/en keys for new or changed page text and require parity tests to pass.

## Risks / Trade-offs

- **Risk:** Automated hard-coded text detection can flag technical identifiers or test data. -> **Mitigation:** use a narrow page/component scan with an explicit allowlist for product names, provider names, ids, npm packages, commands, and test-only fixtures.
- **Risk:** Translating dynamic service errors can hide useful backend detail. -> **Mitigation:** translate frontend-owned wrapper messages and labels, but preserve backend error strings inside bounded error areas when they are the actual diagnostic returned by a service.
- **Risk:** Large locale updates can create inconsistent terminology. -> **Mitigation:** use a small glossary for repeated terms such as Agent, Skill, CLI, SDK, workspace, session, MCP, install, upgrade, downgrade, and rollback.
- **Risk:** Tests that assert exact localized text may become brittle. -> **Mitigation:** update tests to import i18n or assert stable translated values intentionally, not incidental hard-coded language.
