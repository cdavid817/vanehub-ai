## Why

The LSP foundation models "which languages exist" as closed enums (`LanguageFamily`, `ServerKind`), a SQLite `CHECK (language_id IN ('rust', 'typescript_javascript'))` constraint, and a TypeScript tuple literal type. Adding one language today means editing 36 sites across 15 Rust files, five locale bundles, and a table-rebuild migration. Four more languages (Go, Python, C/C++, Java) are planned, so that cost would be paid four times before any of them works.

## What Changes

- Replace the `LanguageFamily` and `ServerKind` closed enums with a static language-definition table keyed by a validated string id, mirroring the shape already proven by `CLI_TOOL_DEFINITIONS` in `contexts/tooling/cli`.
- Move per-language knowledge into that single definition: executable names in preference order, project-root markers, file-extension to LSP `languageId` mapping, default startup arguments, default initialization options, and platform applicability.
- Remove the `LspConfiguration::validate` hard-coded `languages.len() != 2` check and the two-language `Default`.
- Rebuild the `lsp_language_configurations` table without its `CHECK` constraint, preserving existing rows and revisions.
- **BREAKING** Promote startup arguments from a compile-time constant to real per-language configuration. The `startup_arguments_json` column currently only mirrors a `&'static [&'static str]` and no user or command can change it.
- **BREAKING** Widen the frontend `LspLanguageId` type from the literal union `"rust" | "typescript_javascript"` to a runtime set supplied by the backend, and render the settings language cards from that set instead of two hard-coded components.
- Add no new language. Rust and TypeScript/JavaScript behavior stays identical requirement by requirement; the existing native and frontend LSP suites are the acceptance evidence.

## Capabilities

### New Capabilities

None. This change generalizes existing behavior rather than introducing a capability.

### Modified Capabilities

- `lsp-server-management`: the activation requirement stops naming Rust and TypeScript/JavaScript switches individually and instead requires one independent switch per registered language, all defaulting to disabled; the discovery requirement stops naming `rust-analyzer` and `typescript-language-server` as the closed set and instead requires discovery of every registered language's declared executables; startup arguments become declared configuration rather than fixed behavior.
- `settings-center-ui`: the LSP settings requirement stops naming two fixed language switches and instead requires the section to render one card per registered language from the service boundary, keeping the same controls for discovery state, executable override, bounded initialization options, and per-language enablement.

## Impact

**Runtimes affected: desktop and Web.** The Web/mock adapter must widen the same contract so that adapter conformance keeps passing; it still performs no filesystem, process, or network access.

Frontend/backend isolation is unchanged. React components continue to depend only on the `agent-service` boundary, and `invoke()` stays confined to the Tauri adapter. The change is additive at the adapter boundary: one new backend-supplied list replaces two compile-time constants.

Affected code:

- `src-tauri/src/contexts/code_intelligence/domain/{models,configuration}.rs` — the enums, the two-language validation, the constant startup arguments
- `src-tauri/src/contexts/code_intelligence/infrastructure/{schema,configuration_repository,server_discovery,project_root,document_snapshot,server_test,runtime_process_coordinator,semantic_query_coordinator}.rs`
- `src-tauri/src/contexts/code_intelligence/api.rs` and `src-tauri/src/commands/code_intelligence/dto.rs`
- `src/types/lsp.ts`, `src/services/{lsp-contract,web-lsp-client}.ts`, `src/settings/pages/agents/lsp-*.tsx`
- `src/i18n/locales/{en,zh-CN,zh-TW,ja,ko}.json`

Known hazards this change must handle rather than discover late:

- SQLite cannot alter a `CHECK` constraint, so the table must be rebuilt (create, copy, drop, rename) inside one migration.
- A new migration number collides with four hard-coded version assertions that neither the compiler nor clippy reports. All worktrees share one `%APPDATA%\ai.vanehub.app\vanehub.sqlite`, so the number must be chosen after scanning sibling branches.
- The frontend services subtree has an aggregate line budget enforced only by `npm run architecture:check`, which runs after lint, tsc, and build have already passed.
