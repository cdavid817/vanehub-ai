## Why

VaneHub AI already has the correct high-level safety boundary for CLI launch parameters: profiles are typed, raw arguments are rejected, security-sensitive flags are owned by Agent Policies, and Rust owns persistence and process construction. The current implementation nevertheless has three classes of defects that now need one coordinated change:

- The settings page can create invalid empty custom values, can reuse transient custom input across CLIs, always previews the `chat` scope, and reconstructs a shell-like preview in TypeScript instead of displaying backend-produced argv tokens.
- Rust and TypeScript independently define and render the same catalog. The two copies can drift in flags, values, defaults, scopes, descriptions, and provider-specific rendering.
- The catalog cannot express explicit inheritance, negative flags, version gates, dependencies, provider-specific variants, ordered lists, path lists, source audits, or structured diagnostics. A CLI upgrade can therefore make a previously valid profile misleading or invalid without a useful repair path.

The settings page should become a backend-authoritative, version-aware CLI capability editor while preserving the existing rule that VaneHub never accepts arbitrary launch arguments and never lets this page override Agent Policy.

## What Changes

- Replace the hand-maintained Rust and TypeScript catalogs with one native-owned, schema-validated CLI capability registry and a generated TypeScript artifact used only by the Web/mock adapter.
- Replace magic defaults such as the string `default`, `false`, and empty arrays with an explicit `inherit` versus typed `value` selection envelope.
- Add declarative render strategies, argument slots, constraints, version/platform compatibility, dependencies, conflicts, maturity, category, ownership, and official-source audit metadata.
- Add a service-backed draft preview operation with an explicit `chat` or `interactive` scope. The UI displays individual argv tokens and never reconstructs an executable shell command.
- Add structured diagnostics and command-safe error codes instead of parsing English Rust error strings in React.
- Add per-profile revisions and optimistic concurrency so a stale settings window cannot silently overwrite a newer profile.
- Migrate existing SQLite and Web/mock selections to the new envelope without deleting unknown or incompatible rows silently; quarantine invalid legacy rows and provide repair diagnostics.
- Split the native CLI parameter subdomain into domain, application, infrastructure, and published API modules, and make `agent_runtime` consume the tooling context through that API.
- Redesign Settings → CLI Parameters around external CLIs only, with installation/version state, per-CLI dirty/error badges, scoped preview, grouped controls, compatibility badges, filters, policy guidance, and responsive token preview.
- Correct and expand the curated provider catalogs using current official references, including version-gated Claude options, Codex reasoning normalization, Gemini extensions/include directories, and provider-specific OpenCode variants.
- Add contract, migration, interaction, runtime-argv, accessibility, Web/mock parity, and desktop persistence tests.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `cli-parameter-management`: Upgrade catalog authority, typed selection semantics, compatibility evaluation, preview, persistence concurrency, runtime projection, provider coverage, diagnostics, and settings-page behavior.

## Impact

- Frontend types and services: `src/types/`, `src/services/agent-service.ts`, `src/services/tauri-agent-client.ts`, `src/services/web-agent-client.ts`, generated catalog contracts, and related tests.
- Settings UI: `src/settings/pages/cli-parameters-page.tsx`, `src/settings/pages/cli-parameter-control.tsx`, new focused components/hooks under a CLI-parameter page directory, shared settings primitives where appropriate, and all registered locale resources.
- Native tooling context: the current `src-tauri/src/contexts/tooling/cli_parameters.rs` is replaced by a CLI-parameter subdomain with explicit domain/application/infrastructure boundaries and a narrow export through `src-tauri/src/contexts/tooling/api.rs`.
- Native runtime integration: provider invocation builders and Agent Terminal loading consume resolved token segments through the tooling API rather than private storage or catalog functions.
- Native commands and bootstrap: add draft preview, structured DTO/error mapping, and explicit dependency assembly while retaining stable command names for list/save/reset where practical.
- Persistence: add profile metadata/revision state and migrate legacy selection JSON; the existing `enabled` column is retained as a compatibility column but is no longer the source of selection semantics.
- Tooling lifecycle integration: compatibility evaluation reuses the existing CLI detection snapshot and active executable/version rather than spawning new processes during page rendering.
- Documentation and verification: update user-guide parameter documentation, catalog audit records, contract checks, Vitest, Rust tests, Playwright, and desktop fixture coverage.
- No arbitrary raw arguments, credential fields, prompt/system-prompt fields, session identifiers, structured-output flags, approval controls, sandbox controls, or dangerous bypass flags are added.
